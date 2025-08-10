#!/usr/bin/env node

/**
 * MCP Inspector 自動テストスクリプト
 * 
 * 使用方法:
 *   node scripts/test-mcp-inspector.js [server-path]
 * 
 * 例:
 *   node scripts/test-mcp-inspector.js ./target/release/mcp-bookmark
 *   npm test
 */

const { spawn } = require('child_process');
const readline = require('readline');
const fs = require('fs');
const path = require('path');

// カラー出力
const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
};

class MCPServerTester {
  constructor(serverPath) {
    this.serverPath = serverPath || './target/release/mcp-bookmark';
    this.server = null;
    this.requestId = 0;
    this.testResults = [];
    this.startTime = null;
  }

  log(message, color = 'reset') {
    console.log(`${colors[color]}${message}${colors.reset}`);
  }

  async start() {
    this.log('\n🚀 Starting MCP Server...', 'cyan');
    this.startTime = Date.now();

    // サーバー起動
    this.server = spawn(this.serverPath, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, RUST_LOG: 'info' }
    });

    // 標準出力の読み取り設定
    this.rl = readline.createInterface({
      input: this.server.stdout
    });

    // エラー出力の監視
    this.server.stderr.on('data', (data) => {
      if (process.env.DEBUG) {
        console.error(`[STDERR] ${data}`);
      }
    });

    // サーバー終了の監視
    this.server.on('exit', (code) => {
      if (code !== 0 && code !== null) {
        this.log(`Server exited with code ${code}`, 'red');
      }
    });

    // 初期化
    const initResult = await this.sendRequest('initialize', {
      protocolVersion: '1.0.0',
      capabilities: {
        tools: true,
        resources: true
      },
      clientInfo: {
        name: 'mcp-inspector-test',
        version: '1.0.0'
      }
    });

    if (!initResult) {
      throw new Error('Failed to initialize server');
    }

    this.log('✅ Server initialized successfully', 'green');

    // Initialized通知
    await this.sendNotification('initialized');

    return initResult;
  }

  sendRequest(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.requestId;
      const request = {
        jsonrpc: '2.0',
        id,
        method,
        params
      };

      if (process.env.DEBUG) {
        this.log(`→ Request: ${JSON.stringify(request)}`, 'blue');
      }

      // リクエスト送信
      this.server.stdin.write(JSON.stringify(request) + '\n');

      // レスポンス待機
      const timeout = setTimeout(() => {
        this.rl.removeAllListeners('line');
        reject(new Error(`Request timeout for ${method}`));
      }, 10000);

      const handler = (line) => {
        try {
          const response = JSON.parse(line);
          
          if (response.id === id) {
            clearTimeout(timeout);
            this.rl.removeListener('line', handler);
            
            if (process.env.DEBUG) {
              this.log(`← Response: ${JSON.stringify(response)}`, 'green');
            }

            if (response.error) {
              reject(new Error(`${response.error.message} (${response.error.code})`));
            } else {
              resolve(response.result);
            }
          }
        } catch (e) {
          // JSON解析エラーは無視（部分的な出力の可能性）
        }
      };

      this.rl.on('line', handler);
    });
  }

  sendNotification(method, params = {}) {
    const notification = {
      jsonrpc: '2.0',
      method,
      params
    };

    if (process.env.DEBUG) {
      this.log(`→ Notification: ${JSON.stringify(notification)}`, 'blue');
    }

    this.server.stdin.write(JSON.stringify(notification) + '\n');
  }

  async runTest(name, testFn) {
    const startTime = Date.now();
    
    try {
      this.log(`\n📝 ${name}`, 'yellow');
      await testFn();
      const duration = Date.now() - startTime;
      this.log(`   ✅ Passed (${duration}ms)`, 'green');
      this.testResults.push({ name, passed: true, duration });
      return true;
    } catch (error) {
      const duration = Date.now() - startTime;
      this.log(`   ❌ Failed: ${error.message}`, 'red');
      this.testResults.push({ name, passed: false, duration, error: error.message });
      return false;
    }
  }

  async runAllTests() {
    this.log('\n🧪 Running Test Suite...', 'magenta');

    // Test 1: ツール一覧取得
    await this.runTest('Get tool list', async () => {
      const result = await this.sendRequest('tools/list');
      
      if (!result.tools || !Array.isArray(result.tools)) {
        throw new Error('Invalid tools list response');
      }

      const expectedTools = [
        'search_bookmarks',
        'search_bookmarks_fulltext',
        'get_available_profiles',
        'list_bookmark_folders',
        'get_bookmark_content',
        'search_by_content',
        'get_indexing_status'
      ];

      const toolNames = result.tools.map(t => t.name);
      
      for (const expectedTool of expectedTools) {
        if (!toolNames.includes(expectedTool)) {
          throw new Error(`Missing expected tool: ${expectedTool}`);
        }
      }

      this.log(`   Found ${result.tools.length} tools`, 'cyan');
    });

    // Test 2: リソース一覧取得
    await this.runTest('Get resource list', async () => {
      const result = await this.sendRequest('resources/list');
      
      if (!result.resources || !Array.isArray(result.resources)) {
        throw new Error('Invalid resources list response');
      }

      const hasCollection = result.resources.some(r => 
        r.uri === 'bookmarks://collection'
      );

      if (!hasCollection) {
        throw new Error('Missing bookmarks://collection resource');
      }

      this.log(`   Found ${result.resources.length} resources`, 'cyan');
    });

    // Test 3: 基本検索
    await this.runTest('Basic search', async () => {
      const result = await this.sendRequest('tools/call', {
        name: 'search_bookmarks',
        arguments: {
          query: 'test'
        }
      });

      if (!result.content || !Array.isArray(result.content)) {
        throw new Error('Invalid search response');
      }

      this.log(`   Found ${result.content.length} search results`, 'cyan');
    });

    // Test 4: プロファイル一覧
    await this.runTest('Get Chrome profiles', async () => {
      const result = await this.sendRequest('tools/call', {
        name: 'get_available_profiles',
        arguments: {}
      });

      if (!result.content || !Array.isArray(result.content)) {
        throw new Error('Invalid profiles response');
      }

      // JSONテキストをパース
      const profilesText = result.content[0]?.text;
      if (!profilesText) {
        throw new Error('No profile data returned');
      }

      const profiles = JSON.parse(profilesText);
      this.log(`   Found ${profiles.length} Chrome profiles`, 'cyan');
    });

    // Test 5: フォルダ一覧
    await this.runTest('List bookmark folders', async () => {
      const result = await this.sendRequest('tools/call', {
        name: 'list_bookmark_folders',
        arguments: {}
      });

      if (!result.content || !Array.isArray(result.content)) {
        throw new Error('Invalid folders response');
      }

      const foldersText = result.content[0]?.text;
      if (!foldersText) {
        throw new Error('No folder data returned');
      }

      const folders = JSON.parse(foldersText);
      this.log(`   Found ${folders.length} bookmark folders`, 'cyan');
    });

    // Test 6: フルテキスト検索
    await this.runTest('Full-text search with filters', async () => {
      const result = await this.sendRequest('tools/call', {
        name: 'search_bookmarks_fulltext',
        arguments: {
          query: 'javascript',
          limit: 5
        }
      });

      if (!result.content || !Array.isArray(result.content)) {
        throw new Error('Invalid fulltext search response');
      }

      this.log(`   Full-text search returned ${result.content.length} results`, 'cyan');
    });

    // Test 7: インデックス状態
    await this.runTest('Get indexing status', async () => {
      const result = await this.sendRequest('tools/call', {
        name: 'get_indexing_status',
        arguments: {}
      });

      if (!result.content || !Array.isArray(result.content)) {
        throw new Error('Invalid indexing status response');
      }

      const statusText = result.content[0]?.text;
      if (statusText) {
        const status = JSON.parse(statusText);
        this.log(`   Index: ${status.indexed}/${status.total} bookmarks`, 'cyan');
      }
    });

    // Test 8: エラーハンドリング
    await this.runTest('Error handling - invalid tool', async () => {
      try {
        await this.sendRequest('tools/call', {
          name: 'non_existent_tool',
          arguments: {}
        });
        throw new Error('Should have failed with invalid tool');
      } catch (error) {
        if (!error.message.includes('not found') && !error.message.includes('invalid')) {
          throw error;
        }
        this.log('   Error correctly handled', 'cyan');
      }
    });

    // Test 9: リソース読み取り
    await this.runTest('Read resource', async () => {
      const result = await this.sendRequest('resources/read', {
        uri: 'bookmarks://collection'
      });

      if (!result.contents || !Array.isArray(result.contents)) {
        throw new Error('Invalid resource read response');
      }

      const content = result.contents[0];
      if (!content || !content.text) {
        throw new Error('No resource content returned');
      }

      const bookmarks = JSON.parse(content.text);
      this.log(`   Read ${bookmarks.length} bookmarks from collection`, 'cyan');
    });

    // Test 10: パフォーマンステスト
    await this.runTest('Performance - rapid requests', async () => {
      const iterations = 10;
      const times = [];

      for (let i = 0; i < iterations; i++) {
        const start = Date.now();
        await this.sendRequest('tools/call', {
          name: 'search_bookmarks',
          arguments: { query: `test${i}` }
        });
        times.push(Date.now() - start);
      }

      const avg = times.reduce((a, b) => a + b, 0) / times.length;
      const max = Math.max(...times);
      const min = Math.min(...times);

      this.log(`   Avg: ${avg.toFixed(2)}ms, Min: ${min}ms, Max: ${max}ms`, 'cyan');

      if (avg > 1000) {
        throw new Error(`Average response time too high: ${avg}ms`);
      }
    });
  }

  async stop() {
    if (this.server) {
      this.server.stdin.end();
      this.server.kill('SIGTERM');
      
      // 終了を待つ
      await new Promise(resolve => {
        this.server.on('exit', resolve);
        setTimeout(resolve, 1000);
      });
    }
  }

  printSummary() {
    const totalDuration = Date.now() - this.startTime;
    const passed = this.testResults.filter(r => r.passed).length;
    const failed = this.testResults.filter(r => !r.passed).length;

    this.log('\n' + '='.repeat(50), 'cyan');
    this.log('📊 Test Summary', 'magenta');
    this.log('='.repeat(50), 'cyan');

    this.testResults.forEach(result => {
      const icon = result.passed ? '✅' : '❌';
      const color = result.passed ? 'green' : 'red';
      this.log(`${icon} ${result.name} (${result.duration}ms)`, color);
      if (result.error) {
        this.log(`   Error: ${result.error}`, 'yellow');
      }
    });

    this.log('\n' + '-'.repeat(50), 'cyan');
    this.log(`Total: ${this.testResults.length} tests`, 'white');
    this.log(`Passed: ${passed}`, 'green');
    this.log(`Failed: ${failed}`, failed > 0 ? 'red' : 'green');
    this.log(`Duration: ${(totalDuration / 1000).toFixed(2)}s`, 'cyan');
    this.log('='.repeat(50), 'cyan');

    // テスト結果をファイルに保存
    const reportPath = path.join(process.cwd(), 'test-results.json');
    fs.writeFileSync(reportPath, JSON.stringify({
      timestamp: new Date().toISOString(),
      duration: totalDuration,
      passed,
      failed,
      results: this.testResults
    }, null, 2));
    
    this.log(`\n📄 Test report saved to: ${reportPath}`, 'cyan');

    return failed === 0;
  }
}

// メイン実行
async function main() {
  const serverPath = process.argv[2] || './target/release/mcp-bookmark';
  
  // サーバーの存在確認
  if (!fs.existsSync(serverPath)) {
    console.error(`❌ Server not found at: ${serverPath}`);
    console.error('Run: cargo build --release');
    process.exit(1);
  }

  const tester = new MCPServerTester(serverPath);

  try {
    await tester.start();
    await tester.runAllTests();
  } catch (error) {
    tester.log(`\n❌ Fatal error: ${error.message}`, 'red');
    if (process.env.DEBUG) {
      console.error(error.stack);
    }
  } finally {
    await tester.stop();
    const success = tester.printSummary();
    process.exit(success ? 0 : 1);
  }
}

// エラーハンドリング
process.on('unhandledRejection', (error) => {
  console.error('Unhandled rejection:', error);
  process.exit(1);
});

process.on('SIGINT', async () => {
  console.log('\n⚠️  Interrupted by user');
  process.exit(130);
});

// 実行
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});