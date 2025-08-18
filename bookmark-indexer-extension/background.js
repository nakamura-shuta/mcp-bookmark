// Bookmark Indexer - Background Service Worker
// Indexes bookmark content to local Tantivy search engine via Native Messaging

// Send message to native host (mcp-bookmark-native)
function sendToNative(method, params = {}) {
  return new Promise((resolve, reject) => {
    const port = chrome.runtime.connectNative('com.mcp_bookmark');
    const timeoutId = setTimeout(() => reject(new Error('Timeout')), 30000);
    
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

// Open page in actual tab and extract content
async function fetchContent(url) {
  try {
    // Create a new tab with the URL
    const tab = await chrome.tabs.create({ 
      url: url, 
      active: false  // Don't switch to the tab
    });
    
    // Wait for the page to load
    await new Promise(resolve => {
      const listener = (tabId, changeInfo) => {
        if (tabId === tab.id && changeInfo.status === 'complete') {
          chrome.tabs.onUpdated.removeListener(listener);
          // Add extra delay for SPAs like Notion to fully render
          setTimeout(resolve, 5000);
        }
      };
      chrome.tabs.onUpdated.addListener(listener);
      
      // Timeout after 30 seconds
      setTimeout(() => {
        chrome.tabs.onUpdated.removeListener(listener);
        resolve();
      }, 30000);
    });
    
    // Extract content from the page
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => {
        // This runs in the page context
        const title = document.title || '';
        
        // Clone the body and remove script/style elements
        const bodyClone = document.body.cloneNode(true);
        const scripts = bodyClone.querySelectorAll('script, style, noscript, iframe');
        scripts.forEach(el => el.remove());
        
        // Get text content - try different methods
        let content = '';
        
        // For Notion pages, try to get the main content area
        const notionContent = document.querySelector('[class*="notion-page-content"]') || 
                            document.querySelector('[class*="notion-app-inner"]') ||
                            document.querySelector('main') ||
                            bodyClone;
        
        if (notionContent) {
          content = notionContent.innerText || notionContent.textContent || '';
        } else {
          content = bodyClone.innerText || bodyClone.textContent || '';
        }
        
        // Clean up whitespace - NO LIMIT
        content = content
          .replace(/\s+/g, ' ')
          .trim();  // No substring limit - get ALL content
        
        // Also try to get meta description as fallback
        const metaDesc = document.querySelector('meta[name="description"]');
        const description = metaDesc ? metaDesc.getAttribute('content') : '';
        
        console.log(`Extracted from ${document.location.href}:`);
        console.log(`  Title: ${title}`);
        console.log(`  Content length: ${content.length} chars`);
        console.log(`  Has Notion content: ${!!document.querySelector('[class*="notion"]')}`);
        
        return { title, content, description };
      }
    });
    
    // Close the tab
    await chrome.tabs.remove(tab.id);
    
    // Return the extracted content
    if (results && results[0] && results[0].result) {
      const { title, content, description } = results[0].result;
      // Combine description with content if available
      const fullContent = description ? `${description}\n\n${content}` : content;
      console.log(`Fetched content from ${url}:`);
      console.log(`  Title: ${title}`);
      console.log(`  Content length: ${fullContent.length} chars`);
      console.log(`  First 200 chars: ${fullContent.substring(0, 200)}...`);
      return { title, content: fullContent };
    }
    
    console.log(`No content extracted from ${url}`);
    return null;
  } catch (error) {
    console.error(`Failed to fetch ${url}:`, error);
    return null;
  }
}

// Index a single bookmark with specific index
async function indexBookmarkWithIndex(bookmark, indexName) {
  const content = await fetchContent(bookmark.url);
  
  const payload = {
    id: bookmark.id,
    url: bookmark.url,
    title: bookmark.title || content?.title || '',
    content: content?.content || '',
    folder_path: bookmark.folder_path || [],
    date_added: bookmark.dateAdded,
    date_modified: bookmark.dateModified || bookmark.dateAdded,
    index_name: indexName  // Include index name with each bookmark
  };
  
  console.log(`Sending to native host for ${bookmark.url}:`);
  console.log(`  Index: ${indexName}`);
  console.log(`  Content length: ${payload.content.length} chars`);
  
  return sendToNative('index_bookmark', payload);
}

// Index bookmarks from a folder
async function indexFolder(folderId, folderName, indexName) {
  const tree = await chrome.bookmarks.getSubTree(folderId);
  const bookmarks = flattenTree(tree[0]);
  
  // Use the folder name from popup directly
  const finalFolderName = folderName || tree[0].title || 'Bookmarks';
  const finalIndexName = indexName || `Extension_${finalFolderName}`;
  
  console.log(`Indexing folder: "${finalFolderName}" (ID: ${folderId})`);
  console.log(`Index name: "${finalIndexName}"`);
  
  let indexed = 0;
  let failed = 0;
  const total = bookmarks.length;
  
  for (const bookmark of bookmarks) {
    try {
      // Pass index name with each bookmark
      await indexBookmarkWithIndex(bookmark, finalIndexName);
      indexed++;
      console.log(`[${indexed}/${total}] Indexed: ${bookmark.url}`);
      
      // Update progress
      chrome.runtime.sendMessage({
        type: 'progress',
        indexed,
        failed,
        total
      }).catch(() => {});
      
      // Delay between requests to avoid overwhelming the browser
      await new Promise(resolve => setTimeout(resolve, 500));
    } catch (error) {
      failed++;
      console.error(`Failed: ${bookmark.url}`, error);
    }
  }
  
  return { indexed, failed, total };
}

// Flatten bookmark tree into array
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
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  switch (request.type) {
    case 'index_bookmark':
      // Index with manual index name
      indexBookmarkWithIndex(request.bookmark, 'Manual_Index')
        .then(result => sendResponse({ success: true, result }))
        .catch(error => sendResponse({ success: false, error: error.message }));
      return true;
      
    case 'index_folder':
      indexFolder(request.folderId || '0', request.folderName, request.indexName)
        .then(result => sendResponse({ success: true, result }))
        .catch(error => sendResponse({ success: false, error: error.message }));
      return true;
      
    case 'clear_index':
      sendToNative('clear_index', {})
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

console.log('Bookmark Indexer loaded');