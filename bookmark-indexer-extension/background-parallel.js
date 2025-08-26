// Parallel Bookmark Indexer - Background Service Worker
// Integrates parallel content fetching with batch indexing

// Load dependencies (included as separate script tags in manifest)
// Requires: parallel.js, batch-indexer.js

// Main parallel indexing function
async function indexFolderParallel(folderId, folderName, indexName, options = {}) {
  console.log('[Parallel] Starting parallel indexing');
  console.log(`[Parallel] Folder: ${folderName} (${folderId}), Index: ${indexName}`);
  
  // Get bookmark tree
  const tree = await chrome.bookmarks.getSubTree(folderId);
  const bookmarks = flattenTree(tree[0]);
  
  console.log(`[Parallel] Found ${bookmarks.length} bookmarks to process`);
  
  // Initialize components
  const fetcher = new ParallelContentFetcher({
    maxConcurrent: options.maxConcurrent || 5,
    tabTimeout: options.tabTimeout || 30000,
    contentWaitTime: options.contentWaitTime || 5000,
    retryAttempts: options.retryAttempts || 3
  });
  
  const indexer = new BatchIndexer();
  
  try {
    // Connect to native host
    indexer.connect();
    
    // 1. Fetch content in parallel
    console.log('[Parallel] Phase 1: Fetching content...');
    const startTime = Date.now();
    
    const fetchResult = await fetcher.fetchBatch(bookmarks);
    
    const fetchTime = Date.now() - startTime;
    console.log(`[Parallel] Content fetched in ${fetchTime}ms`);
    console.log(`[Parallel] Success: ${fetchResult.successful.length}, Failed: ${fetchResult.failed.length}`);
    
    // 2. Create content map
    const contentMap = new Map();
    fetchResult.successful.forEach(item => {
      contentMap.set(item.bookmark.url, item.content);
    });
    
    // Report failed fetches
    if (fetchResult.failed.length > 0) {
      console.warn('[Parallel] Failed to fetch content for:', fetchResult.failed);
    }
    
    // 3. Send to index in batch
    console.log('[Parallel] Phase 2: Indexing...');
    const indexStartTime = Date.now();
    
    const indexResult = await indexer.sendBatch(bookmarks, contentMap, indexName);
    
    const indexTime = Date.now() - indexStartTime;
    console.log(`[Parallel] Indexed in ${indexTime}ms`);
    
    // 4. Calculate and return results
    const totalTime = Date.now() - startTime;
    const result = {
      total: bookmarks.length,
      indexed: indexResult.success,
      failed: indexResult.failed,
      fetchTime: fetchTime,
      indexTime: indexTime,
      totalTime: totalTime,
      errors: [...fetchResult.failed, ...indexResult.errors]
    };
    
    console.log('[Parallel] Indexing complete:', result);
    
    // Send final progress
    chrome.runtime.sendMessage({
      type: 'complete',
      result: result
    }).catch(() => {});
    
    return result;
    
  } catch (error) {
    console.error('[Parallel] Error during indexing:', error);
    throw error;
    
  } finally {
    // Clean up
    fetcher.abort();
    indexer.disconnect();
  }
}

// Fallback sequential processing for small batches
async function indexFolderSequential(bookmarks, indexName) {
  console.log('[Sequential] Processing', bookmarks.length, 'bookmarks');
  
  let indexed = 0;
  let failed = 0;
  const errors = [];
  
  for (const bookmark of bookmarks) {
    try {
      // Fetch content
      const content = await fetchContent(bookmark.url);
      
      // Send to native host
      await sendToNative('add_bookmark', {
        id: bookmark.id,
        title: bookmark.title || bookmark.name,
        url: bookmark.url,
        folder_path: bookmark.folder_path,
        date_added: String(bookmark.dateAdded || Date.now()),
        date_modified: bookmark.dateModified ? String(bookmark.dateModified) : null,
        content: content.content,
        index_name: indexName
      });
      
      indexed++;
      console.log(`[Sequential] [${indexed}/${bookmarks.length}] Indexed: ${bookmark.url}`);
      
      // Update progress
      chrome.runtime.sendMessage({
        type: 'progress',
        indexed,
        failed,
        total: bookmarks.length
      }).catch(() => {});
      
    } catch (error) {
      failed++;
      errors.push({ url: bookmark.url, error: error.message });
      console.error(`[Sequential] Failed: ${bookmark.url}`, error);
    }
  }
  
  return {
    total: bookmarks.length,
    indexed,
    failed,
    errors
  };
}

// Helper: Fetch single page content
async function fetchContent(url) {
  const tab = await chrome.tabs.create({ url: url, active: false });
  
  try {
    // Wait for load
    await new Promise((resolve, reject) => {
      let attempts = 0;
      const checkStatus = setInterval(async () => {
        attempts++;
        
        try {
          const updatedTab = await chrome.tabs.get(tab.id);
          if (updatedTab.status === 'complete') {
            clearInterval(checkStatus);
            setTimeout(resolve, 5000); // Wait for SPAs
          } else if (attempts > 60) {
            clearInterval(checkStatus);
            reject(new Error('Tab load timeout'));
          }
        } catch (error) {
          clearInterval(checkStatus);
          reject(error);
        }
      }, 500);
    });
    
    // Extract content
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => {
        const title = document.title || '';
        const bodyClone = document.body.cloneNode(true);
        const scripts = bodyClone.querySelectorAll('script, style, noscript, iframe');
        scripts.forEach(el => el.remove());
        
        let content = bodyClone.innerText || bodyClone.textContent || '';
        content = content.replace(/\s+/g, ' ').trim();
        
        return { title, content };
      }
    });
    
    return results[0].result;
    
  } finally {
    // Clean up tab
    await chrome.tabs.remove(tab.id).catch(() => {});
  }
}

// Helper: Send message to native host
function sendToNative(method, params = {}) {
  return new Promise((resolve, reject) => {
    const port = chrome.runtime.connectNative('com.mcp_bookmark');
    const messageId = `msg_${Date.now()}`;
    
    const timeoutId = setTimeout(() => {
      port.disconnect();
      reject(new Error('Native messaging timeout'));
    }, 30000);
    
    port.onDisconnect.addListener(() => {
      clearTimeout(timeoutId);
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
      }
    });
    
    port.onMessage.addListener(response => {
      clearTimeout(timeoutId);
      port.disconnect();
      
      if (response.error) {
        reject(new Error(response.error.message));
      } else {
        resolve(response.result);
      }
    });
    
    port.postMessage({
      jsonrpc: '2.0',
      id: messageId,
      method: method,
      params: params
    });
  });
}

// Flatten bookmark tree (reused from original)
function flattenTree(node, path = []) {
  const results = [];
  
  if (node.url) {
    results.push({
      id: node.id,
      url: node.url,
      title: node.title,
      name: node.title, // Alias
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

// Message handler with parallel/sequential selection
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  switch (request.type) {
    case 'index_folder':
      // Determine processing mode
      chrome.bookmarks.getSubTree(request.folderId || '0').then(tree => {
        const bookmarks = flattenTree(tree[0]);
        const useParallel = request.parallel !== false && bookmarks.length > 2;
        
        console.log(`[Main] Using ${useParallel ? 'parallel' : 'sequential'} processing`);
        
        if (useParallel) {
          // Use parallel processing
          indexFolderParallel(
            request.folderId || '0',
            request.folderName,
            request.indexName,
            request.options || {}
          )
            .then(result => sendResponse({ success: true, result }))
            .catch(error => sendResponse({ success: false, error: error.message }));
        } else {
          // Use sequential processing
          indexFolderSequential(bookmarks, request.indexName)
            .then(result => sendResponse({ success: true, result }))
            .catch(error => sendResponse({ success: false, error: error.message }));
        }
      }).catch(error => {
        sendResponse({ success: false, error: error.message });
      });
      
      return true; // Async response
      
    case 'list_indexes':
      sendToNative('list_indexes', {})
        .then(result => sendResponse({ success: true, result }))
        .catch(error => sendResponse({ success: false, error: error.message }));
      return true;
      
    case 'abort':
      // TODO: Implement abort functionality
      sendResponse({ success: true });
      return false;
  }
});

console.log('[Parallel] Bookmark Indexer with parallel processing loaded');