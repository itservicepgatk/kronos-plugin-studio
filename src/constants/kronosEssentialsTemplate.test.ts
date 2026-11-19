import test from 'node:test';
import assert from 'node:assert/strict';
import { KRONOS_ESSENTIALS_TEMPLATE } from './kronosEssentialsTemplate';

test('KRONOS Essentials template contains all required Rust crate modules and config', () => {
  const filePaths = Object.keys(KRONOS_ESSENTIALS_TEMPLATE.files).sort();
  assert.deepEqual(filePaths, [
    'Cargo.toml',
    'README.md',
    'data/config.json',
    'src/commands.rs',
    'src/effects.rs',
    'src/host.rs',
    'src/lib.rs',
    'src/menu.rs',
    'src/models.rs',
    'src/storage.rs',
  ]);
});

test('KRONOS Essentials Cargo.toml targets cdylib with release size optimizations', () => {
  const cargo = KRONOS_ESSENTIALS_TEMPLATE.files['Cargo.toml'];
  assert.match(cargo, /name = "kronos-essentials"/);
  assert.match(cargo, /crate-type = \["cdylib"\]/);
  assert.match(cargo, /serde_json/);
});

test('KRONOS Essentials config.json is valid JSON with scaled rank home limits (3, 5, 7, 10+)', () => {
  const raw = KRONOS_ESSENTIALS_TEMPLATE.files['data/config.json'];
  const parsed = JSON.parse(raw) as {
    ranks: Array<{ id: string; max_homes: number; unlimited_homes: boolean }>;
    home_icons: string[];
  };
  assert.ok(Array.isArray(parsed.ranks));
  assert.ok(parsed.home_icons.length >= 4);

  const byId = Object.fromEntries(parsed.ranks.map((r) => [r.id, r]));
  assert.equal(byId.player.max_homes, 3);
  assert.equal(byId.vip.max_homes, 5);
  assert.equal(byId.premium.max_homes, 7);
  assert.equal(byId.legend.max_homes, 10);
  assert.equal(byId.owner.unlimited_homes, true);
});

test('KRONOS Essentials exports core commands, GUI clicks, and 20-TPS animation tick hook', () => {
  const lib = KRONOS_ESSENTIALS_TEMPLATE.files['src/lib.rs'];
  const commands = KRONOS_ESSENTIALS_TEMPLATE.files['src/commands.rs'];
  const effects = KRONOS_ESSENTIALS_TEMPLATE.files['src/effects.rs'];

  for (const fnName of ['init', 'cleanup', 'on_tick', 'on_player_command', 'on_menu_click']) {
    assert.ok(lib.includes(`pub extern "C" fn ${fnName}`), `Missing export ${fnName}`);
  }
  for (const cmd of ['/spawn', '/setspawn', '/sethome', '/home', '/homes', '/delhome', '/back', '/tpa', '/warp', '/tpeffect']) {
    assert.ok(commands.includes(`"${cmd}"`), `Missing command ${cmd}`);
  }
  assert.ok(effects.includes('render_warmup_frame'));
  assert.ok(effects.includes('play_arrival_burst'));
});
