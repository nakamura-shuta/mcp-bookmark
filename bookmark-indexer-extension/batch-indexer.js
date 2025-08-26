// Batch Indexer for Native Messaging
// Handles batch communication with Rust backend

class BatchIndexer {
  constructor() {
    this.port = null;
    this.batchId = null;
    this.messageQueue = [];
    this.responseHandlers = new Map();
  }
  
  /**
   * Connect to native messaging host
   */
  connect() {
    if (this.port && this.port.name) {
      return; // Already connected
    }
    
    this.port = chrome.runtime.connectNative('com.mcp_bookmark');
    this.batchId = `batch_${Date.now()}`;
    
    // Handle disconnection
    this.port.onDisconnect.addListener(() => {
      if (chrome.runtime.lastError) {
        console.error('[BatchIndexer] Disconnected:', chrome.runtime.lastError.message);
      }
      this.port = null;
      
      // Reject all pending handlers
      this.responseHandlers.forEach(handler => {
        handler.reject(new Error('Native messaging disconnected'));
      });
      this.responseHandlers.clear();
    });
    
    // Handle messages
    this.port.onMessage.addListener(response => {
      const handler = this.responseHandlers.get(response.id);
      if (handler) {
        clearTimeout(handler.timeoutId);
        this.responseHandlers.delete(response.id);
        
        if (response.error) {
          handler.reject(new Error(response.error.message || response.error));
        } else {
          handler.resolve(response.result);
        }
      }
    });
  }
  
  /**
   * Send message to native host
   */
  sendMessage(method, params = {}) {
    return new Promise((resolve, reject) => {
      if (!this.port) {
        this.connect();
      }
      
      const messageId = `${this.batchId}_${Date.now()}_${Math.random()}`;
      
      // Set timeout
      const timeoutId = setTimeout(() => {
        this.responseHandlers.delete(messageId);
        reject(new Error(`Timeout waiting for response to ${method}`));
      }, 60000);
      
      // Register handler
      this.responseHandlers.set(messageId, { resolve, reject, timeoutId });
      
      // Send message
      this.port.postMessage({
        jsonrpc: '2.0',
        id: messageId,
        method: method,
        params: params
      });
    });
  }
  
  /**
   * Send batch of bookmarks
   */
  async sendBatch(bookmarks, contentMap, indexName) {
    const total = bookmarks.length;
    console.log(`[BatchIndexer] Sending batch of ${total} bookmarks`);
    
    // Start batch
    await this.sendMessage('batch_start', {
      batch_id: this.batchId,
      total: total,
      index_name: indexName,
      timestamp: new Date().toISOString()
    });
    
    // Send each bookmark
    let completed = 0;
    const errors = [];
    
    for (let i = 0; i < bookmarks.length; i++) {
      const bookmark = bookmarks[i];
      const content = contentMap.get(bookmark.url);
      
      if (!content) {
        console.warn(`[BatchIndexer] No content for ${bookmark.url}`);
        errors.push({ url: bookmark.url, error: 'No content fetched' });
        continue;
      }
      
      try {
        await this.sendMessage('batch_add', {
          batch_id: this.batchId,
          index: i,
          bookmark: {
            id: bookmark.id,
            url: bookmark.url,
            name: bookmark.title || bookmark.name,
            folder_path: bookmark.folder_path || [],
            date_added: String(bookmark.dateAdded || Date.now()),
            date_modified: bookmark.dateModified ? String(bookmark.dateModified) : null
          },
          content: content.content || '',
          index_name: indexName
        });
        
        completed++;
        
        // Send progress update every 10 bookmarks
        if (completed % 10 === 0 || completed === total) {
          await this.sendProgress(completed, total, errors.length);
        }
        
      } catch (error) {
        console.error(`[BatchIndexer] Failed to send bookmark ${i}:`, error);
        errors.push({ url: bookmark.url, error: error.message });
      }
    }
    
    // End batch
    const result = await this.sendMessage('batch_end', {
      batch_id: this.batchId,
      index_name: indexName
    });
    
    return {
      success: completed,
      failed: errors.length,
      errors: errors,
      result: result
    };
  }
  
  /**
   * Send progress update
   */
  async sendProgress(completed, total, errorCount = 0) {
    try {
      await this.sendMessage('progress', {
        batch_id: this.batchId,
        completed: completed,
        total: total,
        errors: errorCount
      });
      
      // Also update UI
      chrome.runtime.sendMessage({
        type: 'progress',
        indexed: completed,
        failed: errorCount,
        total: total
      }).catch(() => {
        // Popup might be closed
      });
      
    } catch (error) {
      console.warn('[BatchIndexer] Failed to send progress:', error);
    }
  }
  
  /**
   * Disconnect from native host
   */
  disconnect() {
    if (this.port) {
      this.port.disconnect();
      this.port = null;
    }
    this.responseHandlers.clear();
  }
}

// Export for use
if (typeof module !== 'undefined' && module.exports) {
  module.exports = BatchIndexer;
}