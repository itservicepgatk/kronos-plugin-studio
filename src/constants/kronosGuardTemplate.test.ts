import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { KRONOS_GUARD_TEMPLATE } from './kronosGuardTemplate';

const EXPECTED_FILES = [
  'Cargo.toml',
  'README.md',
  'data/config.json',
  'src/chat_filter.rs',
  'src/commands.rs',
  'src/host.rs',
  'src/inspector.rs',
  'src/lib.rs',
  'src/menu.rs',
  'src/models.rs',
  'src/punishments.rs',
  'src/storage.rs',
];

test('KRONOS Guard standalone folder and template contain all 12 crate files with exact parity', () => {
  assert.equal(KRONOS_GUARD_TEMPLATE.id, 'kronos-guard');
  assert.deepEqual(Object.keys(KRONOS_GUARD_TEMPLATE.files).sort(), [...EXPECTED_FILES].sort());

  const pluginRoot = path.resolve('plugins/kronos-guard');
  for (const rel of EXPECTED_FILES) {
    const diskPath = path.join(pluginRoot, rel);
    assert.ok(fs.existsSync(diskPath), `Missing standalone file on disk: ${rel}`);
    const diskContent = fs.readFileSync(diskPath, 'utf8');
    assert.equal(
      KRONOS_GUARD_TEMPLATE.files[rel],
      diskContent,
      `Template content drifted from standalone folder for ${rel}`,
    );
  }
});

test('KRONOS Guard Cargo.toml and config.json define hierarchy, warn ladder, chat guard, and block log', () => {
  const cargo = KRONOS_GUARD_TEMPLATE.files['Cargo.toml'];
  assert.match(cargo, /name\s*=\s*"kronos-guard"/);
  assert.match(cargo, /crate-type\s*=\s*\["cdylib"\]/);
  assert.match(cargo, /opt-level\s*=\s*"z"/);
  assert.match(cargo, /lto\s*=\s*true/);

  const cfg = JSON.parse(KRONOS_GUARD_TEMPLATE.files['data/config.json']);
  assert.equal(cfg.inspector_item, 'minecraft:spyglass');
  assert.equal(Object.keys(cfg.staff_ranks).length, 20);
  assert.ok(cfg.staff_ranks.player.weight < cfg.staff_ranks.trainee.weight);
  assert.ok(cfg.staff_ranks.trainee.weight < cfg.staff_ranks.helper.weight);
  assert.ok(cfg.staff_ranks.helper.weight < cfg.staff_ranks.moder.weight);
  assert.ok(cfg.staff_ranks.moder.weight < cfg.staff_ranks.stmoder.weight);
  assert.ok(cfg.staff_ranks.stmoder.weight < cfg.staff_ranks.hr_curator.weight);
  assert.ok(cfg.staff_ranks.hr_curator.weight < cfg.staff_ranks.survival_admin.weight);
  assert.ok(cfg.staff_ranks.survival_admin.weight < cfg.staff_ranks.tech_admin.weight);
  assert.ok(cfg.staff_ranks.tech_admin.weight < cfg.staff_ranks.curator.weight);
  assert.ok(cfg.staff_ranks.curator.weight < cfg.staff_ranks.owner.weight);
  assert.equal(cfg.staff_ranks.helper.can_tempban, false);
  assert.equal(cfg.staff_ranks.moder.can_vanish, true);
  assert.equal(cfg.staff_ranks.stmoder.can_rollback, true);
  assert.equal(cfg.staff_ranks.tech_admin.can_ipban, true);

  assert.equal(cfg.warn_escalation.length, 3);
  assert.equal(cfg.chat_filter.block_advertising, true);
  assert.ok(Array.isArray(cfg.chat_filter.whitelisted_domains));
  assert.equal(cfg.block_log.enabled, true);
  assert.equal(cfg.vanish.silent_containers, true);
});

test('KRONOS Guard exports all 14 WASM hooks, moderation commands, 5 Chest GUIs, and Rust unit tests', () => {
  const libRs = KRONOS_GUARD_TEMPLATE.files['src/lib.rs'];
  for (const fnName of [
    'pub extern "C" fn init',
    'pub extern "C" fn cleanup',
    'pub extern "C" fn on_player_pre_login',
    'pub extern "C" fn on_player_join',
    'pub extern "C" fn on_player_leave',
    'pub extern "C" fn on_tick',
    'pub extern "C" fn on_player_chat',
    'pub extern "C" fn on_player_command',
    'pub extern "C" fn on_menu_click',
    'pub extern "C" fn on_block_break',
    'pub extern "C" fn on_block_place',
    'pub extern "C" fn on_container_open',
    'pub extern "C" fn on_container_transfer',
    'pub extern "C" fn on_explosion',
    '#[cfg(test)]',
  ]) {
    assert.ok(libRs.includes(fnName), `Missing export or test block in src/lib.rs: ${fnName}`);
  }

  const commandsRs = KRONOS_GUARD_TEMPLATE.files['src/commands.rs'];
  for (const cmd of [
    '"ban"',
    '"tempban"',
    '"ipban"',
    '"unban"',
    '"mute"',
    '"unmute"',
    '"warn"',
    '"kick"',
    '"vanish"',
    '"freeze"',
    '"history"',
    '"report"',
    '"sc"',
    '"inspect"',
    '"lookup"',
    '"rollback"',
    '"undo"',
  ]) {
    assert.ok(commandsRs.includes(cmd), `Missing command in src/commands.rs: ${cmd}`);
  }

  const menuRs = KRONOS_GUARD_TEMPLATE.files['src/menu.rs'];
  for (const menuId of [
    'kronos_guard_hub',
    'kronos_guard_player:',
    'kronos_guard_history:',
    'kronos_guard_reports',
    'kronos_guard_blocklog',
  ]) {
    assert.ok(menuRs.includes(menuId), `Missing Chest GUI menu in src/menu.rs: ${menuId}`);
  }
});
