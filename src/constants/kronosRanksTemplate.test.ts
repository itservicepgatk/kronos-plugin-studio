import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { KRONOS_RANKS_TEMPLATE } from './kronosRanksTemplate';
import { KRONOS_GUARD_TEMPLATE } from './kronosGuardTemplate';
import { KRONOS_REGIONS_TEMPLATE } from './kronosRegionsTemplate';
import { KRONOS_ESSENTIALS_TEMPLATE } from './kronosEssentialsTemplate';

const EXPECTED_FILES = [
  'Cargo.toml',
  'README.md',
  'data/config.json',
  'src/commands.rs',
  'src/hierarchy.rs',
  'src/host.rs',
  'src/lib.rs',
  'src/menu.rs',
  'src/models.rs',
  'src/storage.rs',
  'src/tab_chat.rs',
];

test('KRONOS Ranks standalone folder and template contain all 11 files with exact parity', () => {
  assert.equal(KRONOS_RANKS_TEMPLATE.id, 'kronos-ranks');
  assert.deepEqual(Object.keys(KRONOS_RANKS_TEMPLATE.files).sort(), [...EXPECTED_FILES].sort());

  const pluginRoot = path.resolve('plugins/kronos-ranks');
  for (const rel of EXPECTED_FILES) {
    const diskPath = path.join(pluginRoot, rel);
    assert.ok(fs.existsSync(diskPath), `Missing standalone file on disk: ${rel}`);
    assert.equal(KRONOS_RANKS_TEMPLATE.files[rel], fs.readFileSync(diskPath, 'utf8'));
  }
});

test('KRONOS Ranks defines 20 total ranks: 1 player + 10 donate + 9 department staff ranks', () => {
  const cfg = JSON.parse(KRONOS_RANKS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{
      id: string;
      category: string;
      department: string;
      weight: number;
      price_rub: number;
      max_assignable_weight: number;
      assignable_departments: string[];
      perks: { max_homes: number; max_regions: number; max_region_volume: number };
    }>;
  };

  assert.equal(cfg.ranks.length, 20);
  const donate = cfg.ranks.filter((r) => r.category === 'donate');
  const staff = cfg.ranks.filter((r) => r.category === 'staff');
  const player = cfg.ranks.filter((r) => r.category === 'player');

  assert.equal(player.length, 1);
  assert.equal(donate.length, 10);
  assert.equal(staff.length, 9);

  // Verify all 5 staff departments are represented
  const depts = new Set(staff.map((r) => r.department));
  for (const d of ['MODERATION', 'RECRUITMENT', 'SURVIVAL', 'TECHNICAL', 'MANAGEMENT']) {
    assert.ok(depts.has(d), `Missing staff department: ${d}`);
  }

  // Verify strict weight progression across donate and staff tracks
  for (let i = 1; i < donate.length; i++) {
    assert.ok(donate[i].weight > donate[i - 1].weight);
    assert.ok(donate[i].perks.max_homes > donate[i - 1].perks.max_homes);
    assert.ok(donate[i].perks.max_regions > donate[i - 1].perks.max_regions);
  }
  for (let i = 1; i < staff.length; i++) {
    assert.ok(staff[i].weight > staff[i - 1].weight);
  }

  // HR Curator can assign MODERATION up to moder (220), cannot assign TECHNICAL
  const hr = cfg.ranks.find((r) => r.id === 'hr_curator')!;
  assert.equal(hr.department, 'RECRUITMENT');
  assert.equal(hr.max_assignable_weight, 220);
  assert.deepEqual(hr.assignable_departments, ['MODERATION']);
});

test('All 4 KRONOS plugins (Ranks, Guard, Regions, Essentials) share the exact same 20 rank IDs and bridge sync', () => {
  const ranksCfg = JSON.parse(KRONOS_RANKS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{ id: string }>;
  };
  const masterIds = ranksCfg.ranks.map((r) => r.id).sort();

  const guardCfg = JSON.parse(KRONOS_GUARD_TEMPLATE.files['data/config.json']) as {
    staff_ranks: Record<string, unknown>;
  };
  const regionsCfg = JSON.parse(KRONOS_REGIONS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{ id: string }>;
  };
  const essentialsCfg = JSON.parse(KRONOS_ESSENTIALS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{ id: string }>;
  };

  assert.deepEqual(Object.keys(guardCfg.staff_ranks).sort(), masterIds);
  assert.deepEqual(regionsCfg.ranks.map((r) => r.id).sort(), masterIds);
  assert.deepEqual(essentialsCfg.ranks.map((r) => r.id).sort(), masterIds);

  // Every dependent plugin reads kronos_ranks_bridge.json
  assert.ok(KRONOS_RANKS_TEMPLATE.files['src/storage.rs'].includes('export_bridge_snapshot'));
  assert.ok(KRONOS_GUARD_TEMPLATE.files['src/storage.rs'].includes('kronos_ranks_bridge.json'));
  assert.ok(KRONOS_REGIONS_TEMPLATE.files['src/storage.rs'].includes('kronos_ranks_bridge.json'));
  assert.ok(KRONOS_ESSENTIALS_TEMPLATE.files['src/storage.rs'].includes('kronos_ranks_bridge.json'));
});

test('KRONOS Ranks exports WASM hooks, cross-plugin FFI queries, 5 GUIs, commands, and Rust unit tests', () => {
  const lib = KRONOS_RANKS_TEMPLATE.files['src/lib.rs'];
  for (const fnName of [
    'pub extern "C" fn init',
    'pub extern "C" fn cleanup',
    'pub extern "C" fn on_player_join',
    'pub extern "C" fn on_player_leave',
    'pub extern "C" fn on_tick',
    'pub extern "C" fn on_player_chat',
    'pub extern "C" fn on_player_command',
    'pub extern "C" fn on_menu_click',
    'pub extern "C" fn kronos_get_rank_weight',
    'pub extern "C" fn kronos_has_permission',
    '#[cfg(test)]',
  ]) {
    assert.ok(lib.includes(fnName), `Missing export/test in kronos-ranks lib.rs: ${fnName}`);
  }

  const menu = KRONOS_RANKS_TEMPLATE.files['src/menu.rs'];
  for (const menuId of [
    'kronos_ranks_donate',
    'kronos_ranks_staff',
    'kronos_ranks_profile:',
    'kronos_ranks_hr',
    'kronos_ranks_audit',
  ]) {
    assert.ok(menu.includes(menuId), `Missing Chest GUI in kronos-ranks menu.rs: ${menuId}`);
  }
});
