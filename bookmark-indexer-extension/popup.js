// Bookmark Indexer - Popup Controller

let selectedFolderId = null;
let selectedFolderName = null;

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
  await loadFolders();
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
    
    console.log(`Indexing folder: "${selectedFolderName}" (ID: ${selectedFolderId})`);
    
    button.disabled = true;
    showProgress();
    showStatus('Indexing...', 'info');
    
    chrome.runtime.sendMessage({
      type: 'index_folder',
      folderId: selectedFolderId,
      folderName: selectedFolderName
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
      } else {
        showStatus(`Error: ${response?.error || 'Unknown error'}`, 'error');
      }
    });
  });
  
  // Index current tab
  document.getElementById('index-current').addEventListener('click', async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    
    if (!tab?.url || !tab.url.startsWith('http')) {
      showStatus('Cannot index this tab', 'error');
      return;
    }
    
    showStatus('Indexing current tab...', 'info');
    
    chrome.runtime.sendMessage({
      type: 'index_bookmark',
      bookmark: {
        id: `tab_${Date.now()}`,
        url: tab.url,
        title: tab.title || 'Untitled',
        folder_path: ['Manual'],
        dateAdded: Date.now()
      }
    }, (response) => {
      if (response?.success) {
        showStatus('Tab indexed successfully!', 'success');
      } else {
        showStatus(`Error: ${response?.error || 'Unknown error'}`, 'error');
      }
    });
  });
  
  // Test connection
  document.getElementById('test').addEventListener('click', () => {
    chrome.runtime.sendMessage({ type: 'test' }, (response) => {
      if (response?.success) {
        showStatus('Connection OK', 'success');
      } else {
        showStatus(`Connection failed: ${response?.error || 'Unknown error'}`, 'error');
      }
    });
  });
  
  // Clear index
  document.getElementById('clear').addEventListener('click', () => {
    if (!confirm('Clear the entire index?')) return;
    
    chrome.runtime.sendMessage({ type: 'clear_index' }, (response) => {
      if (response?.success) {
        showStatus('Index cleared', 'success');
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