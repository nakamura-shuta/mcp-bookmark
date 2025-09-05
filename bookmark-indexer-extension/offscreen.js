// Offscreen document for PDF processing
// This runs in a DOM environment, so PDF.js can work properly

// Initialize PDF.js
pdfjsLib.GlobalWorkerOptions.workerSrc = chrome.runtime.getURL('pdfjs/pdf.worker.js');

// Listen for messages from the background script
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.type === 'extract-pdf-text') {
    extractPdfText(request.url)
      .then(text => sendResponse({ success: true, text }))
      .catch(error => sendResponse({ success: false, error: error.message }));
    return true; // Keep the message channel open for async response
  }
});

async function extractPdfText(url) {
  try {
    console.log(`[Offscreen] Fetching PDF from: ${url}`);
    
    // Fetch the PDF file
    const response = await fetch(url, { 
      credentials: 'omit',  // Avoid CORS issues
      mode: 'cors'
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} ${response.statusText}`);
    }
    
    const arrayBuffer = await response.arrayBuffer();
    console.log(`[Offscreen] Downloaded ${arrayBuffer.byteLength} bytes`);
    
    // Load PDF with PDF.js
    const loadingTask = pdfjsLib.getDocument({
      data: new Uint8Array(arrayBuffer),
      // Worker is available in offscreen document
      disableWorker: false
    });
    
    const pdf = await loadingTask.promise;
    console.log(`[Offscreen] Loaded PDF with ${pdf.numPages} pages`);
    
    let fullText = [];
    
    // Extract text from each page
    for (let pageNum = 1; pageNum <= pdf.numPages; pageNum++) {
      const page = await pdf.getPage(pageNum);
      const textContent = await page.getTextContent();
      
      // Combine text items
      const pageText = textContent.items
        .map(item => item.str)
        .join(' ')
        .trim();
      
      if (pageText) {
        fullText.push(`--- Page ${pageNum} ---\n${pageText}`);
      }
    }
    
    // Clean up
    await pdf.destroy();
    
    const result = fullText.join('\n\n');
    console.log(`[Offscreen] Extracted ${result.length} total characters`);
    
    return result || 'PDF document (no extractable text)';
    
  } catch (error) {
    console.error(`[Offscreen] Error extracting text from ${url}:`, error);
    throw error;
  }
}

console.log('[Offscreen] PDF processor ready');