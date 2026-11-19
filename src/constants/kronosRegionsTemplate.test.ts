import test from 'node:test';
import assert from 'node:assert/strict';
import { KRONOS_REGIONS_TEMPLATE } from './kronosRegionsTemplate';

test('KRONOS Regions template contains all 10 required crate files', () => {
  const paths = Object.keys(KRONOS_REGIONS_TEMPLATE.files).sort();
  assert.deepEqual(paths, [
    'Cargo.toml',
    'README.md',
    'data/config.json',
    'src/commands.rs',
    'src/host.rs',
    'src/lib.rs',
    'src/menu.rs',
    'src/models.rs',
    'src/protection.rs',
    'src/storage.rs',
    'src/visualizer.rs',
  ]);
});

test('KRONOS Regions config.json defines Claim Wand, visualizer settings, and rank progression perks', () => {
  const cfg = JSON.parse(KRONOS_REGIONS_TEMPLATE.files['data/config.json']) as {
    wand_item_id: string;
    wand_tag: string;
    visualizer_duration_seconds: number;
    ranks: Array<{
      id: string;
      max_regions: number;
      max_volume: number;
      can_toggle_pvp: boolean;
      can_toggle_fall_damage: boolean;
      can_toggle_entry: boolean;
      unlimited_regions: boolean;
    }>;
  };

  assert.equal(cfg.wand_item_id, 'minecraft:golden_axe');
  assert.equal(cfg.wand_tag, 'kronos_region_wand');
  assert.ok(cfg.visualizer_duration_seconds >= 15);

  assert.equal(cfg.ranks.length, 20);
  const byId = Object.fromEntries(cfg.ranks.map((r) => [r.id, r]));
  assert.equal(byId.player.max_regions, 2);
  assert.equal(byId.player.can_toggle_pvp, false);
  assert.equal(byId.vip.max_regions, 3);
  assert.equal(byId.vip.can_toggle_pvp, true);
  assert.equal(byId.vip.can_toggle_fall_damage, false);
  assert.equal(byId.premium.max_regions, 4);
  assert.equal(byId.premium.can_toggle_fall_damage, true);
  assert.equal(byId.legend.max_regions, 10);
  assert.equal(byId.legend.can_toggle_entry, true);
  assert.equal(byId.kronos.max_regions, 30);
  assert.equal(byId.survival_admin.unlimited_regions, true);
  assert.equal(byId.owner.unlimited_regions, true);
});

test('KRONOS Regions exports all protection hooks, wand click, 6 GUIs, granular flags, and Rust tests', () => {
  const lib = KRONOS_REGIONS_TEMPLATE.files['src/lib.rs'];
  const menu = KRONOS_REGIONS_TEMPLATE.files['src/menu.rs'];
  const commands = KRONOS_REGIONS_TEMPLATE.files['src/commands.rs'];
  const protection = KRONOS_REGIONS_TEMPLATE.files['src/protection.rs'];
  const viz = KRONOS_REGIONS_TEMPLATE.files['src/visualizer.rs'];

  for (const fnName of [
    'init',
    'cleanup',
    'on_tick',
    'on_player_command',
    'on_wand_click',
    'on_menu_click',
    'on_block_break',
    'on_block_place',
    'on_block_interact',
    'on_explosion',
    'on_pvp_damage',
    'on_fall_damage',
    'on_mob_spawn',
    'on_fire_spread',
  ]) {
    assert.ok(lib.includes(`pub extern "C" fn ${fnName}`), `Missing export ${fnName}`);
  }
  assert.ok(lib.includes('#[cfg(test)]'), 'Missing built-in Rust unit tests');

  for (const menuId of ['rg_list', 'rg_manage:', 'rg_flags:', 'rg_expand:', 'rg_members:', 'rg_themes:', 'rg_confirm_delete:']) {
    assert.ok(menu.includes(menuId), `Missing menu ${menuId}`);
  }

  for (const sub of ['"wand"', '"claim"', '"rename"', '"delete"', '"expand"', '"flag"', '"add"', '"show"']) {
    assert.ok(commands.includes(sub), `Missing command subhandler ${sub}`);
  }

  for (const flag of ['open_chests', 'use_doors', 'use_buttons_levers', 'use_workstations', 'explosions', 'pvp', 'fall_damage']) {
    assert.ok(protection.includes(flag), `Missing granular flag enforcement for ${flag}`);
  }
  assert.ok(viz.includes('render_border_frame') && viz.includes('spawn_point_beacon'));
});
