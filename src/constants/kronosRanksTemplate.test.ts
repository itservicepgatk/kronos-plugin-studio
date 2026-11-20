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
  'src/lib.rs',
  'src/menu.rs',
  'src/models.rs',
  'src/storage.rs',
  'src/tab_chat.rs',
];

test('KRONOS Ranks standalone folder and template contain all 10 files with exact parity (no host.rs)', () => {
  assert.equal(KRONOS_RANKS_TEMPLATE.id, 'kronos-ranks');
  assert.deepEqual(Object.keys(KRONOS_RANKS_TEMPLATE.files).sort(), [...EXPECTED_FILES].sort());
  assert.ok(!('src/host.rs' in KRONOS_RANKS_TEMPLATE.files));

  const pluginRoot = path.resolve('plugins/kronos-ranks');
  assert.ok(!fs.existsSync(path.join(pluginRoot, 'src/host.rs')), 'Obsolete src/host.rs must be removed');
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

test('KRONOS Ranks uses official pumpkin-plugin-api 0.1.0 with zero legacy pumpkin_host FFI', () => {
  const cargo = KRONOS_RANKS_TEMPLATE.files['Cargo.toml'];
  assert.ok(cargo.includes('pumpkin-plugin-api = "0.1.0"'));
  assert.ok(cargo.includes('tracing = "0.1"'));
  assert.ok(cargo.includes('crate-type = ["cdylib"]'));

  // Zero fictitious pumpkin_host FFI or raw C-pointer exports anywhere in kronos-ranks
  for (const [rel, content] of Object.entries(KRONOS_RANKS_TEMPLATE.files)) {
    assert.ok(!content.includes('pumpkin_host'), `Legacy pumpkin_host found in ${rel}`);
    assert.ok(!content.includes('extern "C"'), `Raw extern "C" found in ${rel}`);
    assert.ok(!content.includes('wasm_import_module'), `Legacy wasm_import_module found in ${rel}`);
    assert.ok(!content.includes('read_str('), `Legacy read_str pointer helper found in ${rel}`);
  }

  const lib = KRONOS_RANKS_TEMPLATE.files['src/lib.rs'];
  for (const symbol of [
    'impl Plugin for KronosRanksPlugin',
    'register_plugin!(KronosRanksPlugin)',
    'PlayerJoinEvent',
    'PlayerLeaveEvent',
    'PlayerChatEvent',
    'ServerTickEndEvent',
    'BedrockFormResponseEvent',
    'FS_READ_DATA',
    'FS_WRITE_DATA',
    '#[cfg(test)]',
  ]) {
    assert.ok(lib.includes(symbol), `Missing pumpkin-plugin-api symbol in lib.rs: ${symbol}`);
  }

  const tabChat = KRONOS_RANKS_TEMPLATE.files['src/tab_chat.rs'];
  for (const api of [
    'set_tab_list_name',
    'set_tab_list_order',
    'set_tab_list_header_footer',
    'set_display_name',
    'get_ping',
    'format_chat_line',
    'format_staff_chat_line',
    'SoundCategory::Players',
  ]) {
    assert.ok(tabChat.includes(api), `Missing Tab/Chat API in tab_chat.rs: ${api}`);
  }

  const menu = KRONOS_RANKS_TEMPLATE.files['src/menu.rs'];
  for (const ui of [
    'open_donate_showcase',
    'open_staff_departments',
    'open_player_profile',
    'open_hr_panel',
    'open_audit_menu',
    'click_run_command',
    'click_suggest_command',
    'hover_show_text',
    'SimpleForm',
    'as_bedrock',
  ]) {
    assert.ok(menu.includes(ui), `Missing Java/Bedrock menu API in menu.rs: ${ui}`);
  }

  const commands = KRONOS_RANKS_TEMPLATE.files['src/commands.rs'];
  for (const cmd of [
    'kronos-ranks:player',
    'kronos-ranks:staff',
    'kronos-ranks:admin',
    '"sc"',
    '"staffchat"',
    '"setrank"',
    '"donate"',
    '"ranks"',
    '"staff"',
    'reset_all_ranks',
  ]) {
    assert.ok(commands.includes(cmd), `Missing command/permission in commands.rs: ${cmd}`);
  }
});

test('Cross-plugin rank audit: exact numeric parity across all 20 ranks, dual-track merge, UTF-8 safety, and security guards', () => {
  const ranksCfg = JSON.parse(KRONOS_RANKS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{
      id: string;
      weight: number;
      perks: {
        max_homes: number;
        max_regions: number;
        max_region_volume: number;
        tp_warmup_seconds: number;
        rtp_cooldown_seconds: number;
        border_tier: number;
      };
    }>;
  };
  const guardCfg = JSON.parse(KRONOS_GUARD_TEMPLATE.files['data/config.json']) as {
    staff_ranks: Record<string, { weight: number }>;
  };
  const regionsCfg = JSON.parse(KRONOS_REGIONS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{ id: string; tier: number; max_regions: number; max_volume: number }>;
  };
  const essentialsCfg = JSON.parse(KRONOS_ESSENTIALS_TEMPLATE.files['data/config.json']) as {
    ranks: Array<{
      id: string;
      max_homes: number;
      warmup_seconds: number;
      rtp_cooldown_seconds: number;
    }>;
  };

  const regionsById = Object.fromEntries(regionsCfg.ranks.map((r) => [r.id, r]));
  const essentialsById = Object.fromEntries(essentialsCfg.ranks.map((r) => [r.id, r]));

  for (const rank of ranksCfg.ranks) {
    assert.equal(
      guardCfg.staff_ranks[rank.id].weight,
      rank.weight,
      `Weight mismatch in kronos-guard for ${rank.id}`,
    );
    assert.equal(
      regionsById[rank.id].max_regions,
      rank.perks.max_regions,
      `max_regions mismatch in kronos-regions for ${rank.id}`,
    );
    assert.equal(
      regionsById[rank.id].max_volume,
      rank.perks.max_region_volume,
      `max_region_volume mismatch in kronos-regions for ${rank.id}`,
    );
    assert.ok(
      rank.perks.border_tier <= regionsById[rank.id].tier,
      `border_tier (${rank.perks.border_tier}) exceeds kronos-regions tier (${regionsById[rank.id].tier}) for ${rank.id}`,
    );
    assert.equal(
      essentialsById[rank.id].max_homes,
      rank.perks.max_homes,
      `max_homes mismatch in kronos-essentials for ${rank.id}`,
    );
    assert.equal(
      essentialsById[rank.id].warmup_seconds,
      rank.perks.tp_warmup_seconds,
      `warmup_seconds mismatch in kronos-essentials for ${rank.id}`,
    );
    assert.equal(
      essentialsById[rank.id].rtp_cooldown_seconds,
      rank.perks.rtp_cooldown_seconds,
      `rtp_cooldown_seconds mismatch in kronos-essentials for ${rank.id}`,
    );
  }

  // Disk parity for regions & essentials templates as well
  for (const [dir, tpl] of [
    ['plugins/kronos-regions', KRONOS_REGIONS_TEMPLATE],
    ['plugins/kronos-essentials', KRONOS_ESSENTIALS_TEMPLATE],
  ] as const) {
    for (const [rel, content] of Object.entries(tpl.files)) {
      const disk = fs.readFileSync(path.join(path.resolve(dir), rel), 'utf8');
      assert.equal(content, disk, `Template drift in ${dir}/${rel}`);
    }
  }

  // UTF-8 duration parser safety (no byte-index split_at on Cyrillic duration suffix)
  assert.ok(!KRONOS_RANKS_TEMPLATE.files['src/models.rs'].includes('split_at(raw.len()'));
  assert.ok(!KRONOS_GUARD_TEMPLATE.files['src/models.rs'].includes('split_at(raw.len()'));

  // Security guards in kronos-ranks
  assert.ok(KRONOS_RANKS_TEMPLATE.files['src/commands.rs'].includes('kronos.ranks.'));
  assert.ok(KRONOS_RANKS_TEMPLATE.files['src/hierarchy.rs'].includes('assignable_departments'));

  // Bridge-aware rank resolution across guard & dual-track merge across regions/essentials
  assert.ok(KRONOS_GUARD_TEMPLATE.files['src/commands.rs'].includes('storage::effective_rank'));
  assert.ok(KRONOS_GUARD_TEMPLATE.files['src/chat_filter.rs'].includes('storage::effective_rank'));
  assert.ok(KRONOS_GUARD_TEMPLATE.files['src/menu.rs'].includes('storage::effective_rank'));
  assert.ok(KRONOS_REGIONS_TEMPLATE.files['src/models.rs'].includes('effective_rank_for'));
  assert.ok(KRONOS_REGIONS_TEMPLATE.files['src/models.rs'].includes('next_donate_after'));
  assert.ok(KRONOS_ESSENTIALS_TEMPLATE.files['src/models.rs'].includes('effective_rank_for'));
  assert.ok(KRONOS_ESSENTIALS_TEMPLATE.files['src/models.rs'].includes('next_donate_after'));

  // Zero stale 6-rank references across all 4 plugin crates
  for (const tpl of [KRONOS_RANKS_TEMPLATE, KRONOS_GUARD_TEMPLATE, KRONOS_REGIONS_TEMPLATE, KRONOS_ESSENTIALS_TEMPLATE]) {
    for (const [rel, content] of Object.entries(tpl.files)) {
      assert.ok(
        !content.includes('player|vip|premium|legend|admin|owner'),
        `Stale 6-rank usage string in ${tpl.id}/${rel}`,
      );
      assert.ok(!content.includes('[ADMIN]'), `Stale [ADMIN] prefix in ${tpl.id}/${rel}`);
      assert.ok(!content.includes('next_rank_after'), `Deprecated next_rank_after in ${tpl.id}/${rel}`);
    }
  }
});

