import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { KRONOS_ESSENTIALS_TEMPLATE } from './kronosEssentialsTemplate';

const PLUGIN_ROOT = path.resolve(process.cwd(), 'plugins/kronos-essentials');

test('KRONOS Essentials template contains modular Rust files without fictitious src/host.rs', () => {
  const filePaths = Object.keys(KRONOS_ESSENTIALS_TEMPLATE.files).sort();
  assert.deepEqual(filePaths, [
    'Cargo.toml',
    'README.md',
    'data/config.json',
    'src/commands.rs',
    'src/effects.rs',
    'src/lib.rs',
    'src/menu.rs',
    'src/models.rs',
    'src/storage.rs',
  ]);
  assert.equal(fs.existsSync(path.join(PLUGIN_ROOT, 'src/host.rs')), false);
});

test('KRONOS Essentials template stays byte-for-byte synced with plugins/kronos-essentials', () => {
  for (const [relPath, content] of Object.entries(KRONOS_ESSENTIALS_TEMPLATE.files)) {
    const diskContent = fs.readFileSync(path.join(PLUGIN_ROOT, relPath), 'utf8');
    assert.equal(content, diskContent, `Template file ${relPath} drifted from disk`);
  }
});

test('KRONOS Essentials has zero pumpkin_host FFI or extern "C" imports and uses official pumpkin_plugin_api', () => {
  const cargo = KRONOS_ESSENTIALS_TEMPLATE.files['Cargo.toml'];
  assert.match(cargo, /name = "kronos-essentials"/);
  assert.match(cargo, /crate-type = \["cdylib"\]/);
  assert.match(cargo, /pumpkin-plugin-api = "0\.1\.0"/);
  assert.match(cargo, /serde_json/);
  assert.match(cargo, /tracing/);

  for (const [relPath, content] of Object.entries(KRONOS_ESSENTIALS_TEMPLATE.files)) {
    if (!relPath.endsWith('.rs')) continue;
    assert.ok(!content.includes('pumpkin_host'), `${relPath} must not reference pumpkin_host`);
    assert.ok(!content.includes('extern "C"'), `${relPath} must not declare extern "C" FFI`);
    assert.ok(!content.includes('wasm_import_module'), `${relPath} must not use wasm_import_module`);
  }

  const lib = KRONOS_ESSENTIALS_TEMPLATE.files['src/lib.rs'];
  assert.ok(lib.includes('register_plugin!(KronosEssentialsPlugin);'));
  assert.ok(lib.includes('impl Plugin for KronosEssentialsPlugin'));
  assert.ok(lib.includes('#[cfg(test)]'), 'Missing built-in Rust unit tests');
});

test('KRONOS Essentials implements safe RTP surface scan without hardcoded Y=90', () => {
  const effects = KRONOS_ESSENTIALS_TEMPLATE.files['src/effects.rs'];
  const commands = KRONOS_ESSENTIALS_TEMPLATE.files['src/commands.rs'];

  assert.ok(!commands.includes('y: 90.0'), 'Hardcoded y: 90.0 must be removed from /rtp');
  assert.ok(effects.includes('get_motion_blocking_height'));
  assert.ok(effects.includes('get_top_block_y'));
  assert.ok(effects.includes('get_block_state'));
  assert.ok(effects.includes('get_border().contains'));
  assert.ok(effects.includes('is_block_hazardous'));
  assert.ok(effects.includes('play_custom_sound'));
  assert.ok(effects.includes('SoundCategory::Players'));
  assert.ok(effects.includes('StatusEffectType::Blindness'));
  assert.ok(effects.includes('StatusEffectType::Resistance'));
  assert.ok(effects.includes('StatusEffectType::SlowFalling'));
});

test('KRONOS Essentials implements /sharehome, interactive TextComponent + Bedrock menu, and 15s Anti-Combat', () => {
  const lib = KRONOS_ESSENTIALS_TEMPLATE.files['src/lib.rs'];
  const commands = KRONOS_ESSENTIALS_TEMPLATE.files['src/commands.rs'];
  const menu = KRONOS_ESSENTIALS_TEMPLATE.files['src/menu.rs'];
  const storage = KRONOS_ESSENTIALS_TEMPLATE.files['src/storage.rs'];

  // Anti-Combat events and exact blocking message
  assert.ok(lib.includes('EntityDamageByEntityEvent'));
  assert.ok(lib.includes('EntityDamageEvent'));
  assert.ok(commands.includes('Вы в бою! Телепортация заблокирована на'));

  // Shared homes commands and persistence in essentials_store.json
  assert.ok(commands.includes('"sharehome"'));
  assert.ok(commands.includes('"share"') && commands.includes('"unshare"'));
  assert.ok(storage.includes('essentials_store.json'));
  assert.ok(storage.includes('share_home') && storage.includes('unshare_home'));
  assert.ok(storage.includes('save_store_atomic') && storage.includes('fs::rename'));

  // Interactive chat menu with click/hover + Bedrock form support
  assert.ok(menu.includes('click_run_command'));
  assert.ok(menu.includes('click_suggest_command'));
  assert.ok(menu.includes('hover_show_text'));
  assert.ok(menu.includes('SimpleForm'));
  assert.ok(commands.includes('"menu"') && commands.includes('"homes"'));
});

test('KRONOS Essentials config.json remains valid JSON with 15s combat tag and 20 ranks', () => {
  const raw = KRONOS_ESSENTIALS_TEMPLATE.files['data/config.json'];
  const parsed = JSON.parse(raw) as {
    combat_tag_seconds: number;
    safe_teleport_check: boolean;
    ranks: Array<{ id: string; max_homes: number; unlimited_homes: boolean }>;
  };
  assert.equal(parsed.combat_tag_seconds, 15);
  assert.equal(parsed.safe_teleport_check, true);
  assert.equal(parsed.ranks.length, 20);
});
