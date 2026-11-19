import test from 'node:test';
import assert from 'node:assert/strict';
import { KRONOS_ESSENTIALS_TEMPLATE } from './kronosEssentialsTemplate';

test('KRONOS Essentials v2 template contains all required Rust crate modules and config', () => {
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

test('KRONOS Essentials v2 Cargo.toml targets cdylib v2.0.0 with size optimizations', () => {
  const cargo = KRONOS_ESSENTIALS_TEMPLATE.files['Cargo.toml'];
  assert.match(cargo, /name = "kronos-essentials"/);
  assert.match(cargo, /version = "2\.0\.0"/);
  assert.match(cargo, /crate-type = \["cdylib"\]/);
  assert.match(cargo, /serde_json/);
});

test('KRONOS Essentials v2 config.json validates rank limits, anti-combat, safe tp, and RTP config', () => {
  const raw = KRONOS_ESSENTIALS_TEMPLATE.files['data/config.json'];
  const parsed = JSON.parse(raw) as {
    combat_tag_seconds: number;
    safe_teleport_check: boolean;
    post_teleport_invulnerability_ticks: number;
    autosave_interval_seconds: number;
    rtp: { min_radius: number; max_radius: number; drop_height: number };
    ranks: Array<{
      id: string;
      max_homes: number;
      max_shared_per_home: number;
      can_back_on_death: boolean;
      unlimited_homes: boolean;
    }>;
  };

  assert.equal(parsed.combat_tag_seconds, 15);
  assert.equal(parsed.safe_teleport_check, true);
  assert.ok(parsed.post_teleport_invulnerability_ticks >= 40);
  assert.equal(parsed.autosave_interval_seconds, 30);
  assert.ok(parsed.rtp.max_radius > parsed.rtp.min_radius);
  assert.ok(parsed.rtp.drop_height > 0);

  assert.equal(parsed.ranks.length, 20);
  const byId = Object.fromEntries(parsed.ranks.map((r) => [r.id, r]));
  assert.equal(byId.player.max_homes, 3);
  assert.equal(byId.player.can_back_on_death, false);
  assert.equal(byId.vip.max_homes, 5);
  assert.equal(byId.vip.can_back_on_death, true);
  assert.equal(byId.premium.max_homes, 7);
  assert.equal(byId.legend.max_homes, 16);
  assert.equal(byId.kronos.max_homes, 50);
  assert.equal(byId.survival_admin.unlimited_homes, true);
  assert.equal(byId.owner.unlimited_homes, true);
});

test('KRONOS Essentials v2 exports lifecycle hooks, 5 GUIs, RTP, shared homes, atomic save, and Rust tests', () => {
  const lib = KRONOS_ESSENTIALS_TEMPLATE.files['src/lib.rs'];
  const commands = KRONOS_ESSENTIALS_TEMPLATE.files['src/commands.rs'];
  const menu = KRONOS_ESSENTIALS_TEMPLATE.files['src/menu.rs'];
  const effects = KRONOS_ESSENTIALS_TEMPLATE.files['src/effects.rs'];
  const storage = KRONOS_ESSENTIALS_TEMPLATE.files['src/storage.rs'];

  for (const fnName of [
    'init',
    'cleanup',
    'on_tick',
    'on_player_join',
    'on_player_damage',
    'on_player_death',
    'on_player_command',
    'on_menu_click',
  ]) {
    assert.ok(lib.includes(`pub extern "C" fn ${fnName}`), `Missing export ${fnName}`);
  }
  assert.ok(lib.includes('#[cfg(test)]'), 'Missing built-in Rust unit tests');

  for (const cmd of ['/spawn', '/setspawn', '/sethome', '/home', '/homes', '/delhome', '/rtp', '/back', '/tpa', '/warp', '/tpeffect', '/setrank']) {
    assert.ok(commands.includes(`"${cmd}"`), `Missing command ${cmd}`);
  }
  assert.ok(commands.includes('"share"') && commands.includes('"unshare"') && commands.includes('"fav"'));

  for (const menuId of ['kronos_homes:', 'kronos_confirm_delete:', 'kronos_home_edit:', 'kronos_effects', 'kronos_tpa', 'kronos_warps']) {
    assert.ok(menu.includes(menuId), `Missing menu ${menuId}`);
  }

  assert.ok(effects.includes('play_rtp_meteor_launch'));
  assert.ok(effects.includes('render_meteor_trail_frame'));
  assert.ok(effects.includes('can_use_theme'));
  assert.ok(storage.includes('save_store_atomic') && storage.includes('fs::rename'));
});
