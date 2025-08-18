// Bookmark Indexer - Popup Controller

let selectedFolderId = null;
let selectedFolderName = null;

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
  await loadFolders();
  await loadIndexList();
  setupListeners();
  
  // Listen for progress updates
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === 'progress') {
      updateProgress(msg.indexed, msg.total);
    }
  });
});

// Load bookmark folders
async function loadFolders() {
  const tree = await chrome.bookmarks.getTree();
  const select = document.getElementById('folder-select');
  select.innerHTML = '<option value="">Select a folder...</option>';
  
  function addFolder(node, level = 0) {
    if (!node.children) return;
    
    const count = countBookmarks(node);
    if (count === 0 && level > 0) return;
    
    const option = document.createElement('option');
    option.value = node.id;
    option.dataset.folderName = node.title || 'Bookmarks';
    option.textContent = `${'  '.repeat(level)}${node.title || 'Bookmarks'} (${count})`;
    select.appendChild(option);
    
    if (node.children) {
      node.children.forEach(child => addFolder(child, level + 1));
    }
  }
  
  tree[0].children.forEach(root => addFolder(root, 0));
}

// Count bookmarks in folder
function countBookmarks(node) {
  if (node.url) return 1;
  if (!node.children) return 0;
  return node.children.reduce((sum, child) => sum + countBookmarks(child), 0);
}

// Load existing indexes
async function loadIndexList() {
  try {
    chrome.runtime.sendMessage({ type: 'list_indexes' }, (response) => {
      const listElement = document.getElementById('index-list');
      
      if (response?.success && response.result?.indexes) {
        const indexes = response.result.indexes;
        if (indexes.length === 0) {
          listElement.innerHTML = '<div style="color: #999; font-size: 13px;">No indexes found</div>';
        } else {
          listElement.innerHTML = indexes.map(idx => `
            <div style="padding: 4px 0; border-bottom: 1px solid #eee; font-size: 13px;">
              <div style="font-weight: 500;">${idx.name}</div>
              <div style="color: #666; font-size: 11px;">
                ${idx.doc_count} docs | ${formatSize(idx.size)}
              </div>
            </div>
          `).join('');
        }
      } else {
        listElement.innerHTML = '<div style="color: #999; font-size: 13px;">Failed to load indexes</div>';
      }
    });
  } catch (error) {
    console.error('Failed to load indexes:', error);
  }
}

// Format file size
function formatSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

// Setup event listeners
function setupListeners() {
  // Folder selection
  document.getElementById('folder-select').addEventListener('change', (e) => {
    const select = e.target;
    selectedFolderId = select.value;
    const selectedOption = select.options[select.selectedIndex];
    selectedFolderName = selectedOption ? selectedOption.dataset.folderName : null;
    document.getElementById('index-folder').disabled = !selectedFolderId;
  });
  
  // Index folder
  document.getElementById('index-folder').addEventListener('click', async () => {
    if (!selectedFolderId || !selectedFolderName) return;
    
    const button = document.getElementById('index-folder');
    const customName = document.getElementById('indexName').value.trim();
    
    // Validate custom name
    if (customName && !/^[a-zA-Z0-9_]+$/.test(customName)) {
      showStatus('Index name can only contain letters, numbers, and underscores', 'error');
      return;
    }
    
    // Create index name
    const folderNameSafe = selectedFolderName.replace(/[^a-zA-Z0-9_]/g, '_');
    const indexName = customName 
      ? `${customName}_${folderNameSafe}`
      : `Extension_${folderNameSafe}`;
    
    console.log(`Indexing folder: "${selectedFolderName}" with index name: "${indexName}"`);
    
    button.disabled = true;
    showProgress();
    showStatus('Indexing...', 'info');
    
    chrome.runtime.sendMessage({
      type: 'index_folder',
      folderId: selectedFolderId,
      folderName: selectedFolderName,
      indexName: indexName
    }, (response) => {
      button.disabled = false;
      hideProgress();
      
      if (response?.success) {
        const { indexed, failed } = response.result;
        if (failed > 0) {
          showStatus(`Indexed ${indexed} bookmarks (${failed} failed)`, 'info');
        } else {
          showStatus(`Successfully indexed ${indexed} bookmarks!`, 'success');
        }
        // Reload index list after successful indexing
        loadIndexList();
      } else {
        showStatus(`Error: ${response?.error || 'Unknown error'}`, 'error');
      }
    });
  });
}

// Progress bar
function showProgress() {
  document.getElementById('progress').style.display = 'block';
  updateProgress(0, 100);
}

function hideProgress() {
  setTimeout(() => {
    document.getElementById('progress').style.display = 'none';
  }, 2000);
}

function updateProgress(current, total) {
  const percentage = total > 0 ? (current / total * 100) : 0;
  document.getElementById('progress-fill').style.width = `${percentage}%`;
  document.getElementById('progress-text').textContent = `${current} / ${total}`;
}

// Status message
function showStatus(message, type = 'info') {
  const status = document.getElementById('status');
  status.textContent = message;
  status.className = `status show ${type}`;
  
  setTimeout(() => {
    status.classList.remove('show');
  }, 5000);
}