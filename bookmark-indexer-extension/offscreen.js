// Offscreen document for PDF processing
// This runs in a DOM environment, so PDF.js can work properly

// Initialize PDF.js
pdfjsLib.GlobalWorkerOptions.workerSrc = chrome.runtime.getURL('pdfjs/pdf.worker.js');

// Listen for messages from the background script
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.type === 'extract-pdf-text') {
    extractPdfText(request.url)
      .then(result => sendResponse({ success: true, ...result }))
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

    // Check content type to ensure it's actually a PDF
    const contentType = response.headers.get('content-type');
    if (contentType && !contentType.includes('application/pdf')) {
      console.warn(`[Offscreen] URL is not a PDF (content-type: ${contentType}): ${url}`);
      throw new Error(`Not a PDF file (content-type: ${contentType})`);
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
    let pageOffsets = [];
    let currentOffset = 0;

    // Extract text from each page with page markers
    for (let pageNum = 1; pageNum <= pdf.numPages; pageNum++) {
      const page = await pdf.getPage(pageNum);
      const textContent = await page.getTextContent();

      // Combine text items
      const pageText = textContent.items
        .map(item => item.str)
        .join(' ')
        .trim();

      // Record offset before adding page marker
      pageOffsets.push(currentOffset);

      const pageMarker = `[PAGE:${pageNum}]`;
      const pageContent = pageText ? `${pageMarker}\n${pageText}` : pageMarker;

      fullText.push(pageContent);

      // Update offset: marker + newline + content + double newline (if not last page)
      currentOffset += pageContent.length;
      if (pageNum < pdf.numPages) {
        currentOffset += 2; // for '\n\n' separator
      }
    }

    // Clean up
    await pdf.destroy();

    const text = fullText.join('\n\n');
    console.log(`[Offscreen] Extracted ${text.length} total characters from ${pdf.numPages} pages`);

    // Return text with page metadata
    return {
      text: text || 'PDF document (no extractable text)',
      page_info: {
        page_count: pdf.numPages,
        page_offsets: pageOffsets,
        content_type: 'pdf',
        total_chars: text.length
      }
    };

  } catch (error) {
    console.error(`[Offscreen] Error extracting text from ${url}:`, error);
    throw error;
  }
}

console.log('[Offscreen] PDF processor ready');