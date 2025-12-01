#!/usr/bin/env node

/**
 * CMapファイルをpdfjs-distからChrome拡張のpdfjs/cmapsディレクトリにコピーするスクリプト
 *
 * 使用方法:
 *   pnpm run setup-cmaps
 */

const fs = require('fs');
const path = require('path');

const SOURCE_CMAPS_DIR = path.join(__dirname, '..', 'node_modules', 'pdfjs-dist', 'cmaps');
const TARGET_CMAPS_DIR = path.join(__dirname, '..', 'pdfjs', 'cmaps');
const SOURCE_FONTS_DIR = path.join(__dirname, '..', 'node_modules', 'pdfjs-dist', 'standard_fonts');
const TARGET_FONTS_DIR = path.join(__dirname, '..', 'pdfjs', 'standard_fonts');

// 日本語PDF処理に必要な主要なCMapファイル
// （全ファイルをコピーするとサイズが大きくなるため、必要なものだけ選択）
const REQUIRED_CMAPS = [
  // Adobe-Japan1（日本語CIDフォント用）
  'Adobe-Japan1-0.bcmap',
  'Adobe-Japan1-1.bcmap',
  'Adobe-Japan1-2.bcmap',
  'Adobe-Japan1-3.bcmap',
  'Adobe-Japan1-4.bcmap',
  'Adobe-Japan1-5.bcmap',
  'Adobe-Japan1-6.bcmap',
  'Adobe-Japan1-UCS2.bcmap',

  // UniJIS（Unicode-JIS変換用）
  'UniJIS-UCS2-H.bcmap',
  'UniJIS-UCS2-V.bcmap',
  'UniJIS-UCS2-HW-H.bcmap',
  'UniJIS-UCS2-HW-V.bcmap',
  'UniJIS-UTF16-H.bcmap',
  'UniJIS-UTF16-V.bcmap',
  'UniJIS-UTF32-H.bcmap',
  'UniJIS-UTF32-V.bcmap',
  'UniJIS2004-UTF16-H.bcmap',
  'UniJIS2004-UTF16-V.bcmap',
  'UniJIS2004-UTF32-H.bcmap',
  'UniJIS2004-UTF32-V.bcmap',
  'UniJISPro-UCS2-V.bcmap',
  'UniJISPro-UCS2-HW-V.bcmap',
  'UniJISPro-UTF8-V.bcmap',
  'UniJISX0213-UTF32-H.bcmap',
  'UniJISX0213-UTF32-V.bcmap',
  'UniJISX02132004-UTF32-H.bcmap',
  'UniJISX02132004-UTF32-V.bcmap',

  // 90ms-RKSJ（Shift-JIS系）
  '90ms-RKSJ-H.bcmap',
  '90ms-RKSJ-V.bcmap',
  '90msp-RKSJ-H.bcmap',
  '90msp-RKSJ-V.bcmap',
  '90pv-RKSJ-H.bcmap',
  '90pv-RKSJ-V.bcmap',

  // 83pv-RKSJ
  '83pv-RKSJ-H.bcmap',

  // EUC-JP系
  'EUC-H.bcmap',
  'EUC-V.bcmap',

  // H/V（基本的な水平/垂直書き）
  'H.bcmap',
  'V.bcmap',

  // 78系（JIS X 0208-1978）
  '78-EUC-H.bcmap',
  '78-EUC-V.bcmap',
  '78-H.bcmap',
  '78-V.bcmap',
  '78-RKSJ-H.bcmap',
  '78-RKSJ-V.bcmap',
  '78ms-RKSJ-H.bcmap',
  '78ms-RKSJ-V.bcmap',

  // Add系
  'Add-H.bcmap',
  'Add-V.bcmap',
  'Add-RKSJ-H.bcmap',
  'Add-RKSJ-V.bcmap',

  // Ext系（拡張文字用）
  'Ext-H.bcmap',
  'Ext-V.bcmap',
  'Ext-RKSJ-H.bcmap',
  'Ext-RKSJ-V.bcmap',

  // Hankaku（半角）
  'Hankaku.bcmap',
  'Hiragana.bcmap',
  'Katakana.bcmap',

  // Roman
  'Roman.bcmap',

  // NWP系
  'NWP-H.bcmap',
  'NWP-V.bcmap',

  // RKSJ系
  'RKSJ-H.bcmap',
  'RKSJ-V.bcmap',

  // WP系
  'WP-Symbol.bcmap',

  // Identity
  'Identity-H.bcmap',
  'Identity-V.bcmap',
];

function copyFile(src, dest) {
  try {
    fs.copyFileSync(src, dest);
    return true;
  } catch (error) {
    console.error(`  Error copying ${path.basename(src)}: ${error.message}`);
    return false;
  }
}

function copyDirectory(srcDir, destDir, filter = () => true) {
  if (!fs.existsSync(srcDir)) {
    return { copied: 0, skipped: 0, error: `Source not found: ${srcDir}` };
  }

  if (!fs.existsSync(destDir)) {
    fs.mkdirSync(destDir, { recursive: true });
  }

  const files = fs.readdirSync(srcDir);
  let copied = 0;
  let skipped = 0;

  for (const filename of files) {
    if (!filter(filename)) continue;

    const srcPath = path.join(srcDir, filename);
    const destPath = path.join(destDir, filename);

    if (fs.existsSync(destPath)) {
      const srcStat = fs.statSync(srcPath);
      const destStat = fs.statSync(destPath);
      if (srcStat.size === destStat.size) {
        skipped++;
        continue;
      }
    }

    if (copyFile(srcPath, destPath)) {
      copied++;
    }
  }

  return { copied, skipped };
}

function main() {
  console.log('PDF.jsリソースのセットアップを開始します...\n');

  // ソースディレクトリの確認
  if (!fs.existsSync(SOURCE_CMAPS_DIR)) {
    console.error(`Error: Source directory not found: ${SOURCE_CMAPS_DIR}`);
    console.error('Please run "pnpm install" first.');
    process.exit(1);
  }

  // ターゲットディレクトリの作成
  if (!fs.existsSync(TARGET_CMAPS_DIR)) {
    fs.mkdirSync(TARGET_CMAPS_DIR, { recursive: true });
    console.log(`Created directory: ${TARGET_CMAPS_DIR}`);
  }

  // 利用可能なCMapファイルを取得
  const availableFiles = fs.readdirSync(SOURCE_CMAPS_DIR);
  console.log(`利用可能なCMapファイル: ${availableFiles.length}個\n`);

  // 必要なファイルをコピー
  let copied = 0;
  let skipped = 0;
  let notFound = 0;

  console.log('CMapファイルをコピー中（全ファイル）...');

  // 全CMapファイルをコピー（日本語以外のCJKも含む）
  for (const filename of availableFiles) {
    if (!filename.endsWith('.bcmap')) continue;

    const srcPath = path.join(SOURCE_CMAPS_DIR, filename);
    const destPath = path.join(TARGET_CMAPS_DIR, filename);

    if (fs.existsSync(destPath)) {
      // 既存ファイルのサイズを比較
      const srcStat = fs.statSync(srcPath);
      const destStat = fs.statSync(destPath);
      if (srcStat.size === destStat.size) {
        skipped++;
        continue;
      }
    }

    if (copyFile(srcPath, destPath)) {
      copied++;
    }
  }

  // 標準フォントのコピー
  console.log('\n標準フォントをコピー中...');
  const fontsResult = copyDirectory(SOURCE_FONTS_DIR, TARGET_FONTS_DIR, (f) => f.endsWith('.pfb'));
  if (fontsResult.error) {
    console.warn(`  Warning: ${fontsResult.error}`);
  } else {
    console.log(`  コピー: ${fontsResult.copied}ファイル, スキップ: ${fontsResult.skipped}ファイル`);
  }

  // 結果表示
  console.log('\n--- セットアップ完了 ---');
  console.log(`CMapコピー: ${copied}ファイル`);
  console.log(`CMapスキップ（既存）: ${skipped}ファイル`);
  if (notFound > 0) {
    console.log(`見つからない: ${notFound}ファイル`);
  }

  // ターゲットディレクトリのサイズを計算
  const cmapFiles = fs.readdirSync(TARGET_CMAPS_DIR);
  let cmapSize = 0;
  for (const filename of cmapFiles) {
    const stat = fs.statSync(path.join(TARGET_CMAPS_DIR, filename));
    cmapSize += stat.size;
  }
  console.log(`\nCMap: ${cmapFiles.length}ファイル (${(cmapSize / 1024).toFixed(1)} KB)`);

  if (fs.existsSync(TARGET_FONTS_DIR)) {
    const fontFiles = fs.readdirSync(TARGET_FONTS_DIR);
    let fontSize = 0;
    for (const filename of fontFiles) {
      const stat = fs.statSync(path.join(TARGET_FONTS_DIR, filename));
      fontSize += stat.size;
    }
    console.log(`Fonts: ${fontFiles.length}ファイル (${(fontSize / 1024).toFixed(1)} KB)`);
  }

  console.log(`\n出力先: ${path.dirname(TARGET_CMAPS_DIR)}`);
}

main();
