#!/usr/bin/env node

/**
 * MCP Protocol Test
 * Usage: node scripts/mcp-test.js [server-path]
 */

const { spawn } = require('child_process');
const readline = require('readline');

class MCPTester {
  constructor(serverPath = './target/release/mcp-bookmark') {
    this.serverPath = serverPath;
    this.server = null;
    this.requestId = 0;
  }

  log(message, status = '') {
    const icons = { ok: '✓', error: '✗', info: 'ℹ' };
    const prefix = icons[status] || '';
    console.log(`${prefix} ${message}`);
  }

  async start() {
    this.log('Starting MCP Server...', 'info');
    
    this.server = spawn(this.serverPath, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, RUST_LOG: 'error' }
    });

    this.rl = readline.createInterface({
      input: this.server.stdout
    });

    // Initialize
    const init = await this.sendRequest('initialize', {
      protocolVersion: '1.0.0',
      capabilities: { tools: true },
      clientInfo: { name: 'test', version: '1.0' }
    });

    if (!init) throw new Error('Failed to initialize');
    this.log('Server initialized', 'ok');

    await this.sendNotification('initialized');
    return init;
  }

  sendRequest(method, params = {}) {
    return new Promise((resolve) => {
      const id = ++this.requestId;
      const request = JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params
      });

      const timeout = setTimeout(() => resolve(null), 3000);
      
      const handler = (line) => {
        try {
          const response = JSON.parse(line);
          if (response.id === id) {
            clearTimeout(timeout);
            this.rl.removeListener('line', handler);
            resolve(response);
          }
        } catch {}
      };

      this.rl.on('line', handler);
      this.server.stdin.write(request + '\n');
    });
  }

  sendNotification(method, params = {}) {
    const notification = JSON.stringify({
      jsonrpc: '2.0',
      method,
      params
    });
    this.server.stdin.write(notification + '\n');
  }

  async testTools() {
    this.log('Testing tools/list...', 'info');
    const tools = await this.sendRequest('tools/list');
    
    if (tools?.result?.tools) {
      this.log(`Found ${tools.result.tools.length} tools`, 'ok');
      tools.result.tools.forEach(t => 
        console.log(`  - ${t.name}: ${t.description?.substring(0, 50)}...`)
      );
      return true;
    }
    
    this.log('Failed to get tools', 'error');
    return false;
  }

  async testSearch() {
    this.log('Testing bookmark search...', 'info');
    const result = await this.sendRequest('tools/call', {
      name: 'search_bookmarks',
      arguments: { query: 'test', limit: 3 }
    });

    if (result?.result) {
      this.log('Search completed', 'ok');
      return true;
    }
    
    this.log('Search failed', 'error');
    return false;
  }

  async stop() {
    if (this.server) {
      this.server.kill();
      this.log('Server stopped', 'info');
    }
  }

  async run() {
    try {
      console.log('\n🧪 MCP Protocol Test\n' + '='.repeat(30));
      
      await this.start();
      await this.testTools();
      await this.testSearch();
      
      console.log('\n' + '='.repeat(30));
      this.log('All tests completed\n', 'ok');
    } catch (error) {
      this.log(`Error: ${error.message}`, 'error');
    } finally {
      await this.stop();
    }
  }
}

// Run tests
const tester = new MCPTester(process.argv[2]);
tester.run();