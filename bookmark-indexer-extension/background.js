// PDF processing using Offscreen API
let offscreenCreated = false;

async function ensureOffscreenDocument() {
  if (offscreenCreated) return;
  
  // Check if offscreen document already exists
  const existingContexts = await chrome.runtime.getContexts({
    contextTypes: ['OFFSCREEN_DOCUMENT']
  });
  
  if (existingContexts.length > 0) {
    offscreenCreated = true;
    return;
  }
  
  // Create offscreen document
  await chrome.offscreen.createDocument({
    url: 'offscreen.html',
    reasons: ['DOM_PARSER'],
    justification: 'Parse PDFs using PDF.js which requires DOM APIs'
  });
  
  offscreenCreated = true;
  console.log('[Background] Offscreen document created for PDF processing');
}

// Bookmark Indexer - Background Service Worker with Parallel Processing
class ParallelContentFetcher {
  constructor(options = {}) {
    this.maxConcurrent = options.maxConcurrent || 5;
    this.tabTimeout = options.tabTimeout || 30000;
    this.contentWaitTime = options.contentWaitTime || 5000;
    this.retryAttempts = options.retryAttempts || 2;
    this.progressCallback = options.progressCallback || null;
    
    this.activeJobs = new Map();
    this.queue = [];
    this.isRunning = false;
    
    this.metrics = {
      totalProcessed: 0,
      successCount: 0,
      errorCount: 0,
      startTime: null,
      endTime: null,
      errors: [],
      total: 0
    };
  }
  
  sendProgressUpdate() {
    if (this.progressCallback) {
      this.progressCallback(this.metrics.successCount, this.metrics.total, this.metrics.errorCount);
    }
  }
  
  async fetchBatch(bookmarks) {
    console.log(`[Parallel] Starting fetch for ${bookmarks.length} bookmarks`);
    
    if (!bookmarks || bookmarks.length === 0) {
      return this.createEmptyResult();
    }
    
    // Set total for progress tracking
    this.metrics.total = bookmarks.length;
    
    // For 1-2 bookmarks, use sequential processing
    if (bookmarks.length <= 2) {
      console.log(`[Parallel] Using sequential processing for ${bookmarks.length} bookmarks`);
      return this.fetchSequential(bookmarks);
    }
    
    // Start parallel processing
    this.metrics.startTime = Date.now();
    this.isRunning = true;
    
    const actualConcurrent = Math.min(this.maxConcurrent, bookmarks.length);
    this.maxConcurrent = actualConcurrent;
    console.log(`[Parallel] Using ${actualConcurrent} concurrent tabs`);
    
    const promises = bookmarks.map(bookmark => 
      this.enqueue(bookmark.url, bookmark)
    );
    
    this.processQueue();
    
    const results = await Promise.allSettled(promises);
    
    this.isRunning = false;
    this.metrics.endTime = Date.now();
    
    return this.collectResults(results, bookmarks);
  }
  
  async fetchSequential(bookmarks) {
    this.metrics.startTime = Date.now();
    const successful = [];
    const failed = [];
    
    for (const bookmark of bookmarks) {
      try {
        const content = await this.fetchSingleWithRetry(bookmark.url);
        successful.push({ bookmark, content });
        this.metrics.successCount++;
      } catch (error) {
        failed.push({ bookmark, error: error.message });
        this.metrics.errorCount++;
        this.metrics.errors.push({ url: bookmark.url, error: error.message });
      }
      this.metrics.totalProcessed++;
      this.sendProgressUpdate();
    }
    
    this.metrics.endTime = Date.now();
    
    return {
      successful,
      failed,
      metrics: this.metrics
    };
  }
  
  enqueue(url, bookmark) {
    return new Promise((resolve, reject) => {
      this.queue.push({ url, bookmark, resolve, reject });
    });
  }
  
  async processQueue() {
    while (this.isRunning && (this.queue.length > 0 || this.activeJobs.size > 0)) {
      // 最大並列数に達していない場合、新しいジョブを開始
      if (this.activeJobs.size < this.maxConcurrent && this.queue.length > 0) {
        const job = this.queue.shift();
        // 並列実行のため、awaitせずに起動
        this.startJob(job);
      }
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
  
  async startJob(job) {
    const { url, bookmark, resolve, reject } = job;
    
    // タブ作成前に仮のIDでジョブを登録（並列数制御のため）
    const tempId = `temp_${Date.now()}_${Math.random()}`;
    this.activeJobs.set(tempId, { url, bookmark, resolve, reject });
    
    let tabId = null;
    
    try {
      const tab = await chrome.tabs.create({ url: url, active: false });
      tabId = tab.id;
      
      // 仮IDを実際のtabIdに置き換え
      const jobData = this.activeJobs.get(tempId);
      this.activeJobs.delete(tempId);
      
      const timeoutId = setTimeout(() => {
        this.handleTimeout(tabId, url);
      }, this.tabTimeout);
      
      this.activeJobs.set(tabId, { 
        url, 
        bookmark, 
        timeoutId, 
        resolve: jobData.resolve, 
        reject: jobData.reject 
      });
      
      await this.waitForTabLoad(tabId);
      const content = await this.extractContent(tabId);
      
      this.cleanupJob(tabId);
      this.metrics.successCount++;
      this.metrics.totalProcessed++;
      this.sendProgressUpdate();
      resolve(content);
      
    } catch (error) {
      console.error(`[Parallel] Error processing ${url}:`, error);
      this.cleanupJob(tabId);
      this.metrics.errorCount++;
      this.metrics.totalProcessed++;
      this.sendProgressUpdate();
      reject(error);
    }
  }
  
  waitForTabLoad(tabId) {
    return new Promise((resolve, reject) => {
      let attempts = 0;
      const maxAttempts = 60;
      
      const checkStatus = async () => {
        attempts++;
        
        try {
          const tab = await chrome.tabs.get(tabId);
          
          if (tab.status === 'complete') {
            setTimeout(resolve, this.contentWaitTime);
          } else if (attempts >= maxAttempts) {
            reject(new Error('Tab load timeout'));
          } else {
            setTimeout(checkStatus, 500);
          }
        } catch (error) {
          reject(error);
        }
      };
      
      checkStatus();
    });
  }
  
  // Helper function to check if URL is a PDF (including local files)
  isPDFUrl(url) {
    if (!url) return false;
    const lowerUrl = url.toLowerCase();
    return lowerUrl.endsWith('.pdf') || 
           lowerUrl.includes('/pdf/') ||
           lowerUrl.includes('?format=pdf') ||
           lowerUrl.includes('&type=pdf') ||
           lowerUrl.includes('application/pdf') ||
           (url.startsWith('file:///') && lowerUrl.includes('.pdf'));
  }

  async extractContent(tabId) {
    // Get tab information to check if it's a PDF
    const tab = await chrome.tabs.get(tabId);
    const url = tab.url;

    // Check if URL is a PDF
    if (this.isPDFUrl(url)) {
      console.log(`[Parallel] PDF detected: ${url}`);

      try {
        // Extract PDF text using PDF.js (returns {text, page_info})
        const pdfResult = await this.extractPdfText(url);
        console.log(`[Parallel] Extracted ${pdfResult.text.length} chars from PDF`);

        return {
          title: tab.title || 'PDF Document',
          url: url,
          content: pdfResult.text,
          description: `PDF Document: ${tab.title}`,
          isPDF: true,
          page_info: pdfResult.page_info
        };
      } catch (error) {
        console.error(`[Parallel] Failed to extract PDF text:`, error);
        // Fallback: return with minimal content
        return {
          title: tab.title || 'PDF Document',
          url: url,
          content: `PDF: ${tab.title} - Could not extract text`,
          description: 'PDF Document',
          isPDF: true
        };
      }
    }
    
    // Regular web page extraction (unchanged)
    const results = await chrome.scripting.executeScript({
      target: { tabId },
      func: () => {
        const title = document.title || '';
        
        const bodyClone = document.body.cloneNode(true);
        const removeElements = bodyClone.querySelectorAll(
          'script, style, noscript, iframe, svg, canvas'
        );
        removeElements.forEach(el => el.remove());
        
        let content = '';
        
        const contentArea = 
          document.querySelector('[class*="notion-page-content"]') || 
          document.querySelector('[class*="notion-app-inner"]') ||
          document.querySelector('main') ||
          document.querySelector('article') ||
          document.querySelector('[role="main"]') ||
          document.querySelector('.content') ||
          bodyClone;
        
        if (contentArea) {
          content = contentArea.innerText || contentArea.textContent || '';
        }
        
        content = content.replace(/\s+/g, ' ').trim();
        
        const metaDesc = document.querySelector('meta[name="description"]');
        const description = metaDesc ? metaDesc.getAttribute('content') : '';
        
        return {
          title,
          content,
          description,
          url: document.location.href,
          isPDF: false
        };
      }
    });
    
    return results[0].result;
  }
  
  handleTimeout(tabId, url) {
    const job = this.activeJobs.get(tabId);
    if (job) {
      console.error(`[Parallel] Timeout for ${url}`);
      this.cleanupJob(tabId);
      job.reject(new Error(`Timeout loading ${url}`));
    }
  }
  
  cleanupJob(tabId) {
    if (!tabId) return;
    
    const job = this.activeJobs.get(tabId);
    if (job) {
      clearTimeout(job.timeoutId);
      this.activeJobs.delete(tabId);
    }
    
    chrome.tabs.remove(tabId).catch(() => {});
  }
  
  // Extract text from PDF using offscreen document
  async extractPdfText(url) {
    try {
      console.log(`[PDF] Processing PDF with offscreen document: ${url}`);

      // Ensure offscreen document exists
      await ensureOffscreenDocument();

      // Send message to offscreen document to extract PDF text
      return new Promise((resolve, reject) => {
        chrome.runtime.sendMessage(
          { type: 'extract-pdf-text', url },
          (response) => {
            if (chrome.runtime.lastError) {
              console.error(`[PDF] Error: ${chrome.runtime.lastError.message}`);
              reject(new Error(chrome.runtime.lastError.message));
            } else if (response && response.success) {
              console.log(`[PDF] Successfully extracted ${response.text.length} characters from ${response.page_info?.page_count || 0} pages`);
              // Return both text and page_info
              resolve({
                text: response.text,
                page_info: response.page_info
              });
            } else {
              console.error(`[PDF] Failed to extract text: ${response?.error || 'Unknown error'}`);
              reject(new Error(response?.error || 'Failed to extract PDF text'));
            }
          }
        );
      });

    } catch (error) {
      console.error(`[PDF] Error extracting text from ${url}:`, error);
      throw error;
    }
  }
  
  async fetchSingleWithRetry(url) {
    let lastError = null;
    
    for (let attempt = 1; attempt <= this.retryAttempts; attempt++) {
      try {
        const tab = await chrome.tabs.create({ url, active: false });
        await this.waitForTabLoad(tab.id);
        const content = await this.extractContent(tab.id);
        await chrome.tabs.remove(tab.id).catch(() => {});
        return content;
      } catch (error) {
        lastError = error;
        console.log(`[Parallel] Attempt ${attempt}/${this.retryAttempts} failed for ${url}`);
        
        if (attempt < this.retryAttempts) {
          await new Promise(r => setTimeout(r, Math.pow(2, attempt) * 1000));
        }
      }
    }
    
    throw lastError;
  }
  
  collectResults(results, bookmarks) {
    const successful = [];
    const failed = [];
    
    results.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        successful.push({
          bookmark: bookmarks[index],
          content: result.value
        });
      } else {
        failed.push({
          bookmark: bookmarks[index],
          error: result.reason.message
        });
      }
    });
    
    return {
      successful,
      failed,
      metrics: this.metrics
    };
  }
  
  createEmptyResult() {
    return {
      successful: [],
      failed: [],
      metrics: {
        totalProcessed: 0,
        successCount: 0,
        errorCount: 0,
        startTime: Date.now(),
        endTime: Date.now(),
        errors: []
      }
    };
  }
  
  abort() {
    console.log('[Parallel] Aborting all jobs');
    this.isRunning = false;
    
    this.activeJobs.forEach((job, tabId) => {
      this.cleanupJob(tabId);
      job.reject(new Error('Processing aborted'));
    });
    
    this.queue.forEach(job => {
      job.reject(new Error('Processing aborted'));
    });
    this.queue = [];
  }
  
  getStatus() {
    return {
      activeJobs: this.activeJobs.size,
      queueLength: this.queue.length,
      isRunning: this.isRunning,
      metrics: this.metrics
    };
  }
}

// Native Communication
function sendToNative(method, params = {}) {
  return new Promise((resolve, reject) => {
    const port = chrome.runtime.connectNative('com.mcp_bookmark');
    const timeoutId = setTimeout(() => reject(new Error('Timeout')), 60000);
    
    port.onDisconnect.addListener(() => {
      clearTimeout(timeoutId);
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
      }
    });
    
    port.onMessage.addListener((response) => {
      clearTimeout(timeoutId);
      port.disconnect();
      response.error ? reject(new Error(response.error.message)) : resolve(response.result);
    });
    
    port.postMessage({
      jsonrpc: '2.0',
      id: `req_${Date.now()}`,
      method: method,
      params: params
    });
  });
}

// Main indexing function
async function indexFolderParallel(folderId, folderName, indexName) {
  const tree = await chrome.bookmarks.getSubTree(folderId);
  const bookmarks = flattenTree(tree[0]);
  
  const finalFolderName = folderName || tree[0].title || 'Bookmarks';
  const finalIndexName = indexName || `Extension_${finalFolderName}`;
  
  console.log(`[Simple] Indexing folder: "${finalFolderName}" (ID: ${folderId})`);
  console.log(`[Simple] Index name: "${finalIndexName}"`);
  console.log(`[Simple] Total bookmarks: ${bookmarks.length}`);
  
  // Check if any bookmarks are PDFs and ensure offscreen document is ready
  const hasPDFs = bookmarks.some(b => b.url && b.url.toLowerCase().endsWith('.pdf'));
  if (hasPDFs) {
    console.log(`[Simple] PDFs detected, preparing offscreen document...`);
    await ensureOffscreenDocument();
  }
  
  try {
    // Step 1: Fetch all content in parallel
    const fetcher = new ParallelContentFetcher({
      maxConcurrent: bookmarks.length <= 2 ? 1 : 5,
      tabTimeout: 30000,
      contentWaitTime: 5000,
      progressCallback: (indexed, total, failed) => {
        // Send progress update to popup
        chrome.runtime.sendMessage({
          type: 'progress',
          indexed: indexed,
          total: total,
          failed: failed,
          skipped: 0
        }).catch(() => {});
      }
    });
    
    const results = await fetcher.fetchBatch(bookmarks);
    
    console.log(`[Simple] Fetch completed:`);
    console.log(`  Successful: ${results.successful.length}`);
    console.log(`  Failed: ${results.failed.length}`);
    console.log(`  Duration: ${results.metrics.endTime - results.metrics.startTime}ms`);
    
    // Step 2: Prepare all bookmark data with content
    const bookmarksWithContent = [];

    for (const { bookmark, content } of results.successful) {
      const bookmarkData = {
        id: bookmark.id,
        url: bookmark.url,
        title: bookmark.title || content?.title || '',
        folder_path: bookmark.folder_path || [],
        date_added: bookmark.dateAdded,
        date_modified: bookmark.dateModified || bookmark.dateAdded,
        content: content?.description ?
          `${content.description}\n\n${content.content}` :
          content?.content || '',
        isPDF: content?.isPDF || false
      };

      // Add page_info if available (for PDFs)
      if (content?.page_info) {
        bookmarkData.page_info = content.page_info;
      }

      bookmarksWithContent.push(bookmarkData);
    }
    
    // Add failed items with empty content
    for (const { bookmark } of results.failed) {
      bookmarksWithContent.push({
        id: bookmark.id,
        url: bookmark.url,
        title: bookmark.title || '',
        folder_path: bookmark.folder_path || [],
        date_added: bookmark.dateAdded,
        date_modified: bookmark.dateModified || bookmark.dateAdded,
        content: ''
      });
    }
    
    // Step 3: Send ALL bookmarks in ONE message
    console.log(`[Simple] Sending ${bookmarksWithContent.length} bookmarks in single message...`);
    
    const indexResult = await sendToNative('index_bookmarks_batch', {
      index_name: finalIndexName,
      bookmarks: bookmarksWithContent
    });
    
    console.log(`[Simple] Indexing completed:`, indexResult);
    
    // Send final progress update to ensure UI shows 100%
    chrome.runtime.sendMessage({
      type: 'progress',
      indexed: results.successful.length,
      failed: results.failed.length,
      skipped: 0,
      total: bookmarks.length
    }).catch(() => {});
    
    return {
      indexed: results.successful.length,
      failed: results.failed.length,
      skipped: 0,
      total: bookmarks.length,
      duration: results.metrics.endTime - results.metrics.startTime
    };
    
  } catch (error) {
    console.error(`[Simple] Indexing failed:`, error);
    throw error;
  }
}

// Helper function to flatten bookmark tree
function flattenTree(node, path = []) {
  const results = [];
  
  if (node.url) {
    results.push({
      id: node.id,
      url: node.url,
      title: node.title,
      folder_path: path,
      dateAdded: node.dateAdded,
      dateModified: node.dateModified
    });
  }
  
  if (node.children) {
    for (const child of node.children) {
      const childPath = node.title ? [...path, node.title] : path;
      results.push(...flattenTree(child, childPath));
    }
  }
  
  return results;
}

// Message handler
chrome.runtime.onMessage.addListener((request, _, sendResponse) => {
  switch (request.type) {
    case 'index_folder':
      // Use simplified parallel indexing
      indexFolderParallel(request.folderId || '0', request.folderName, request.indexName)
        .then(result => sendResponse({ success: true, result }))
        .catch(error => sendResponse({ success: false, error: error.message }));
      return true;
      
    case 'list_indexes':
      sendToNative('list_indexes', {})
        .then(result => sendResponse({ success: true, result }))
        .catch(error => sendResponse({ success: false, error: error.message }));
      return true;
  }
});

console.log('Bookmark Indexer with Simplified Parallel Processing loaded');