#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

// Récupère la nouvelle version depuis l'argument
const newVersion = process.argv[2];
if (!newVersion) {
  console.error('Usage: node bump-version.js <nouvelle-version>');
  process.exit(1);
}

const files = [
  'package.json',
  path.join('src-tauri', 'tauri.conf.json'),
  path.join('src-tauri', 'Cargo.toml'),
];

files.forEach(filePath => {
  const text = fs.readFileSync(filePath, 'utf-8');
  let updated;

  if (filePath.endsWith('.json')) {
    const json = JSON.parse(text);
    if (json.version !== undefined) {
      json.version = newVersion;
      updated = JSON.stringify(json, null, 2) + '\n';
      console.log(`✅ Mis à jour version dans ${filePath}`);
    } else {
      // Sur la conf Tauri, si version absente, on ajoute
      json.package = json.package || {};
      json.package.version = newVersion;
      updated = JSON.stringify(json, null, 2) + '\n';
      console.log(`✅ Ajout version dans ${filePath}`);
    }
  } else if (filePath.endsWith('Cargo.toml')) {
    // remplace la ligne version = "x.y.z"
    updated = text.replace(
      /^version\s*=\s*"[0-9A-Za-z\.\-+]+"$/m,
      `version = "${newVersion}"`
    );
    console.log(`✅ Mis à jour version dans Cargo.toml`);
  }

  if (updated) {
    fs.writeFileSync(filePath, updated, 'utf-8');
  }
});