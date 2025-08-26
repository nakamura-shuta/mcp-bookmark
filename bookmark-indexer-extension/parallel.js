// Parallel Content Fetcher for Chrome Bookmarks
// Handles concurrent tab processing with safety measures

class ParallelContentFetcher {
  constructor(options = {}) {
    // Configuration
    this.maxConcurrent = options.maxConcurrent || 5;
    this.tabTimeout = options.tabTimeout || 30000;
    this.contentWaitTime = options.contentWaitTime || 5000;
    this.retryAttempts = options.retryAttempts || 3;
    
    // State management
    this.activeJobs = new Map();  // tabId -> {url, bookmark, timeoutId, resolve, reject}
    this.queue = [];               // [{url, bookmark, resolve, reject}]
    this.isRunning = false;
    
    // Metrics
    this.metrics = {
      totalProcessed: 0,
      successCount: 0,
      errorCount: 0,
      startTime: null,
      endTime: null,
      errors: []
    };
  }
  
  /**
   * Main entry point for batch processing
   */
  async fetchBatch(bookmarks) {
    console.log(`[Parallel] Starting fetch for ${bookmarks.length} bookmarks`);
    
    // Handle edge cases
    if (!bookmarks || bookmarks.length === 0) {
      return this.createEmptyResult();
    }
    
    // For 1-2 bookmarks, use sequential processing
    if (bookmarks.length <= 2) {
      console.log(`[Parallel] Using sequential processing for ${bookmarks.length} bookmarks`);
      return this.fetchSequential(bookmarks);
    }
    
    // Start parallel processing
    this.metrics.startTime = Date.now();
    this.isRunning = true;
    
    // Adjust concurrency based on bookmark count
    const actualConcurrent = Math.min(this.maxConcurrent, bookmarks.length);
    this.maxConcurrent = actualConcurrent;
    console.log(`[Parallel] Using ${actualConcurrent} concurrent tabs`);
    
    // Create promises for all bookmarks
    const promises = bookmarks.map(bookmark => 
      this.enqueue(bookmark.url, bookmark)
    );
    
    // Start processing queue
    this.processQueue();
    
    // Wait for all to complete
    const results = await Promise.allSettled(promises);
    
    this.isRunning = false;
    this.metrics.endTime = Date.now();
    
    // Collect results
    return this.collectResults(results, bookmarks);
  }
  
  /**
   * Sequential processing for small batches
   */
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
    }
    
    this.metrics.endTime = Date.now();
    
    return {
      successful,
      failed,
      metrics: this.metrics
    };
  }
  
  /**
   * Add bookmark to processing queue
   */
  enqueue(url, bookmark) {
    return new Promise((resolve, reject) => {
      this.queue.push({ url, bookmark, resolve, reject });
    });
  }
  
  /**
   * Process queue with concurrency control
   */
  async processQueue() {
    while (this.isRunning && (this.queue.length > 0 || this.activeJobs.size > 0)) {
      // Start new jobs up to max concurrent
      while (this.activeJobs.size < this.maxConcurrent && this.queue.length > 0) {
        const job = this.queue.shift();
        this.startJob(job);
      }
      
      // Wait before checking again
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
  
  /**
   * Start individual job
   */
  async startJob(job) {
    const { url, bookmark, resolve, reject } = job;
    let tabId = null;
    
    try {
      // Create tab
      const tab = await chrome.tabs.create({ 
        url: url, 
        active: false
      });
      tabId = tab.id;
      
      // Set timeout
      const timeoutId = setTimeout(() => {
        this.handleTimeout(tabId, url);
      }, this.tabTimeout);
      
      // Register job
      this.activeJobs.set(tabId, { 
        url, 
        bookmark, 
        timeoutId, 
        resolve, 
        reject 
      });
      
      // Wait for load
      await this.waitForTabLoad(tabId);
      
      // Extract content
      const content = await this.extractContent(tabId);
      
      // Success
      this.cleanupJob(tabId);
      resolve(content);
      
    } catch (error) {
      console.error(`[Parallel] Error processing ${url}:`, error);
      this.cleanupJob(tabId, error);
      reject(error);
    }
  }
  
  /**
   * Wait for tab to fully load
   */
  waitForTabLoad(tabId) {
    return new Promise((resolve, reject) => {
      let attempts = 0;
      const maxAttempts = 60; // 30 seconds max
      
      const checkStatus = async () => {
        attempts++;
        
        try {
          const tab = await chrome.tabs.get(tabId);
          
          if (tab.status === 'complete') {
            // Additional wait for SPAs
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
  
  /**
   * Extract content from loaded tab
   */
  async extractContent(tabId) {
    const results = await chrome.scripting.executeScript({
      target: { tabId },
      func: () => {
        // Extract page content
        const title = document.title || '';
        
        // Clone body and clean
        const bodyClone = document.body.cloneNode(true);
        const removeElements = bodyClone.querySelectorAll(
          'script, style, noscript, iframe, svg, canvas'
        );
        removeElements.forEach(el => el.remove());
        
        // Get text content
        let content = '';
        
        // Try to find main content area
        const contentArea = 
          document.querySelector('main') ||
          document.querySelector('article') ||
          document.querySelector('[role="main"]') ||
          document.querySelector('.content') ||
          bodyClone;
        
        if (contentArea) {
          content = contentArea.innerText || contentArea.textContent || '';
        }
        
        // Clean whitespace
        content = content.replace(/\s+/g, ' ').trim();
        
        // Get meta description
        const metaDesc = document.querySelector('meta[name="description"]');
        const description = metaDesc ? metaDesc.getAttribute('content') : '';
        
        return {
          title,
          content,
          description,
          url: document.location.href
        };
      }
    });
    
    return results[0].result;
  }
  
  /**
   * Handle job timeout
   */
  handleTimeout(tabId, url) {
    const job = this.activeJobs.get(tabId);
    if (job) {
      console.error(`[Parallel] Timeout for ${url}`);
      this.cleanupJob(tabId, new Error(`Timeout: ${url}`));
      job.reject(new Error(`Timeout loading ${url}`));
    }
  }
  
  /**
   * Clean up completed/failed job
   */
  cleanupJob(tabId, error = null) {
    if (!tabId) return;
    
    const job = this.activeJobs.get(tabId);
    if (job) {
      clearTimeout(job.timeoutId);
      this.activeJobs.delete(tabId);
    }
    
    // Close tab
    chrome.tabs.remove(tabId).catch(() => {
      // Tab may already be closed
    });
  }
  
  /**
   * Fetch single URL with retry
   */
  async fetchSingleWithRetry(url) {
    let lastError = null;
    
    for (let attempt = 1; attempt <= this.retryAttempts; attempt++) {
      try {
        // Create tab and fetch content
        const tab = await chrome.tabs.create({ url, active: false });
        
        // Wait for load
        await this.waitForTabLoad(tab.id);
        
        // Extract content
        const content = await this.extractContent(tab.id);
        
        // Clean up
        await chrome.tabs.remove(tab.id).catch(() => {});
        
        return content;
        
      } catch (error) {
        lastError = error;
        console.log(`[Parallel] Attempt ${attempt}/${this.retryAttempts} failed for ${url}`);
        
        if (attempt < this.retryAttempts) {
          // Exponential backoff
          await new Promise(r => setTimeout(r, Math.pow(2, attempt) * 1000));
        }
      }
    }
    
    throw lastError;
  }
  
  /**
   * Collect and format results
   */
  collectResults(results, bookmarks) {
    const successful = [];
    const failed = [];
    
    results.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        successful.push({
          bookmark: bookmarks[index],
          content: result.value
        });
        this.metrics.successCount++;
      } else {
        failed.push({
          bookmark: bookmarks[index],
          error: result.reason.message
        });
        this.metrics.errorCount++;
        this.metrics.errors.push({
          url: bookmarks[index].url,
          error: result.reason.message
        });
      }
      this.metrics.totalProcessed++;
    });
    
    return {
      successful,
      failed,
      metrics: this.metrics
    };
  }
  
  /**
   * Create empty result
   */
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
  
  /**
   * Abort all active jobs
   */
  abort() {
    console.log('[Parallel] Aborting all jobs');
    this.isRunning = false;
    
    // Cancel all active jobs
    this.activeJobs.forEach((job, tabId) => {
      this.cleanupJob(tabId, new Error('Aborted'));
      job.reject(new Error('Processing aborted'));
    });
    
    // Clear queue
    this.queue.forEach(job => {
      job.reject(new Error('Processing aborted'));
    });
    this.queue = [];
  }
  
  /**
   * Get current status
   */
  getStatus() {
    return {
      activeJobs: this.activeJobs.size,
      queueLength: this.queue.length,
      isRunning: this.isRunning,
      metrics: this.metrics
    };
  }
}

// Export for use in background.js
if (typeof module !== 'undefined' && module.exports) {
  module.exports = ParallelContentFetcher;
}