import fs from 'node:fs';

const version = process.env.APP_VERSION;

if (!version) {
  throw new Error('APP_VERSION is missing. Set APP_VERSION before running this script.');
}

const packagePath = 'package.json';
const tauriConfigPath = 'src-tauri/tauri.conf.json';
const cargoTomlPath = 'src-tauri/Cargo.toml';

const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
packageJson.version = version;
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, 'utf8'));
tauriConfig.version = version;
fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
const updatedCargoToml = cargoToml.replace(
  /(\[package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/,
  `$1${version}$2`,
);

if (updatedCargoToml === cargoToml) {
  throw new Error('Failed to update [package] version in src-tauri/Cargo.toml');
}

fs.writeFileSync(cargoTomlPath, updatedCargoToml);
console.log(`Applied version ${version} to package metadata.`);
