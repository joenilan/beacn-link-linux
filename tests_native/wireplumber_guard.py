#!/usr/bin/env python3
"""Offline predicate and integration checks for the WirePlumber guard."""

from __future__ import annotations

import ctypes
import ctypes.util
import hashlib
import os
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION = ROOT / "integrations" / "wireplumber" / "scripts" / "linx"
SCRIPT = INTEGRATION / "node-state-stream.lua"
CONFIG = (
    ROOT
    / "integrations"
    / "wireplumber"
    / "wireplumber.conf.d"
    / "90-linx-state-stream.conf"
)
PINNED_STOCK_SHA256 = "97761d3098d0bf5aa825e1cd3d096a859d4c1d5fbe5fa7a56fbf794116e95e97"
INLINE_GUARD = '''-- LinX addition is GPL-3.0-only; see the repository's LICENSE.
local target_cache_guard = {}
function target_cache_guard.is_linx_temporary_target (target_key, target_properties)
  if target_key ~= "target.object" or type (target_properties) ~= "table" then
    return false
  end

  if target_properties ["media.class"] ~= "Audio/Sink" or
      target_properties ["linx.temporary-target"] ~= "1" then
    return false
  end

  local name = target_properties ["node.name"]
  if type (name) ~= "string" or
      target_properties ["beacn.link.owner"] ~= name then
    return false
  end

  local pid, nonce = string.match (name,
      "^linx%-desktop%-(%d+)%-([%x]+)$")
  return pid ~= nil and #nonce == 32
end
'''
READINESS_BLOCK = '''
-- The native LinX owner waits for this marker before opening a route. Keep the
-- manager alive; it emits object-added for the existing object on activation
-- and for future default-metadata objects.
-- LinX addition is GPL-3.0-only; see the repository's LICENSE.
linx_cache_guard_metadata_om = ObjectManager {
  Interest {
    type = "metadata",
    Constraint { "metadata.name", "=", "default", type = "pw-global" },
  },
}
linx_cache_guard_metadata_om:connect ("object-added", function (_, metadata)
  metadata:set (0, "linx.target-cache-guard", "Spa:String", "1")
end)
linx_cache_guard_metadata_om:activate ()
'''


def _reconstruct_stock_source() -> bytes:
    derived = SCRIPT.read_bytes().decode("utf-8")
    if not derived.endswith(READINESS_BLOCK):
        raise AssertionError("readiness hook is not the expected final script addition")
    reconstructed = derived[: -len(READINESS_BLOCK)]

    target_guard = (
        '        -- LinX addition is GPL-3.0-only; see the repository\'s LICENSE.\n'
        '        if target_cache_guard.is_linx_temporary_target (\n'
        '            target_key, target_node.properties) then\n'
        '          log:info (node, "preserving saved stream target for LinX temporary sink")\n'
        '          return\n'
        '        end\n'
    )
    if reconstructed.count(target_guard) != 1:
        raise AssertionError("target-save guard is missing or appears more than once")
    reconstructed = reconstructed.replace(target_guard, "", 1)

    if reconstructed.count(INLINE_GUARD) != 1:
        raise AssertionError("inline target guard is missing or appears more than once")
    reconstructed = reconstructed.replace(INLINE_GUARD, "", 1)
    return reconstructed.encode("utf-8")


def _find_lua54() -> ctypes.CDLL:
    candidates = [ctypes.util.find_library("lua5.4"), ctypes.util.find_library("lua54")]
    candidates.extend(("liblua5.4.so.0", "liblua5.4.so", "lua54.dll", "lua5.4.dll"))
    for candidate in filter(None, candidates):
        try:
            return ctypes.CDLL(candidate)
        except OSError:
            continue
    raise unittest.SkipTest("Lua 5.4 shared library is unavailable")


def _run_lua(source: str) -> None:
    lua = _find_lua54()
    lua.luaL_newstate.restype = ctypes.c_void_p
    lua.luaL_openlibs.argtypes = [ctypes.c_void_p]
    lua.luaL_loadstring.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lua.luaL_loadstring.restype = ctypes.c_int
    lua.lua_pcallk.argtypes = [
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_ssize_t,
        ctypes.c_void_p,
    ]
    lua.lua_pcallk.restype = ctypes.c_int
    lua.lua_tolstring.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p]
    lua.lua_tolstring.restype = ctypes.c_char_p
    lua.lua_close.argtypes = [ctypes.c_void_p]

    state = lua.luaL_newstate()
    if not state:
        raise RuntimeError("luaL_newstate failed")
    try:
        lua.luaL_openlibs(state)
        result = lua.luaL_loadstring(state, source.encode("utf-8"))
        if result == 0:
            result = lua.lua_pcallk(state, 0, 0, 0, 0, None)
        if result != 0:
            message = lua.lua_tolstring(state, -1, None)
            raise AssertionError(message.decode("utf-8", "replace") if message else "Lua failed")
    finally:
        lua.lua_close(state)


class WirePlumberGuardTests(unittest.TestCase):
    def test_stock_source_matches_pinned_wireplumber_058_hash(self) -> None:
        digest = hashlib.sha256(_reconstruct_stock_source()).hexdigest()
        self.assertEqual(digest, PINNED_STOCK_SHA256)

    def test_derived_script_only_changes_target_guard_and_readiness(self) -> None:
        stock = _reconstruct_stock_source().decode("utf-8")
        expected = stock.replace(
            'cutils = require ("common-utils")\n',
            'cutils = require ("common-utils")\n'
            + INLINE_GUARD,
            1,
        )
        expected = expected.replace(
            '      if target_node then\n'
            '        target_name = target_node.properties ["node.name"]\n',
            '      if target_node then\n'
            '        -- LinX addition is GPL-3.0-only; see the repository\'s LICENSE.\n'
            '        if target_cache_guard.is_linx_temporary_target (\n'
            '            target_key, target_node.properties) then\n'
            '          log:info (node, "preserving saved stream target for LinX temporary sink")\n'
            '          return\n'
            '        end\n'
            '        target_name = target_node.properties ["node.name"]\n',
            1,
        )
        expected += READINESS_BLOCK
        self.assertEqual(SCRIPT.read_text(encoding="utf-8"), expected)

    def test_guard_predicate_in_lua(self) -> None:
        fixture = f"""
{INLINE_GUARD}
local valid = {{
  ["media.class"] = "Audio/Sink",
  ["linx.temporary-target"] = "1",
  ["node.name"] = "linx-desktop-1234-0123456789abcdef0123456789abcdef",
  ["beacn.link.owner"] = "linx-desktop-1234-0123456789abcdef0123456789abcdef",
}}
assert (target_cache_guard.is_linx_temporary_target ("target.object", valid))
assert (not target_cache_guard.is_linx_temporary_target ("target.node", valid))
local function rejected (key, field, value)
  local props = {{}}
  for k, v in pairs (valid) do props [k] = v end
  props [field] = value
  assert (not target_cache_guard.is_linx_temporary_target (key, props))
end
rejected ("target.object", "media.class", "Stream/Output/Audio")
rejected ("target.object", "linx.temporary-target", nil)
rejected ("target.object", "beacn.link.owner", "foreign-owner")
rejected ("target.object", "node.name", "linx-desktop-1234-deadbeef")
rejected ("target.object", "node.name", "linx-desktop-x-0123456789abcdef0123456789abcdef")
print ("WirePlumber guard predicate: 7 cases passed")
"""
        _run_lua(fixture)

    def test_patch_and_component_override_are_narrow(self) -> None:
        script = SCRIPT.read_text(encoding="utf-8")
        config = CONFIG.read_text(encoding="utf-8")
        self.assertIn(INLINE_GUARD, script)
        self.assertIn("target_cache_guard.is_linx_temporary_target (", script)
        self.assertIn('Constraint { "metadata.name", "=", "default", type = "pw-global" }', script)
        self.assertIn('linx_cache_guard_metadata_om:activate ()', script)
        self.assertIn('metadata:set (0, "linx.target-cache-guard", "Spa:String", "1")', script)
        self.assertIn('{ name = "node/state-stream.lua" }', config)
        self.assertIn('name = linx/node-state-stream.lua', config)
        self.assertNotIn("wireplumber.profiles", config)
        self.assertNotIn('require ("linx.target-cache-guard")', script)

    def test_readiness_manager_marks_existing_and_future_metadata(self) -> None:
        fixture = f"""
local default_metadata = {{}}
function default_metadata:set (subject, key, spa_type, value)
  self.subject, self.key, self.spa_type, self.value = subject, key, spa_type, value
end
local manager = {{}}
function manager:connect (signal, callback)
  assert (signal == "object-added")
  self.callback = callback
end
function manager:activate ()
  self.callback (self, default_metadata)
end
function ObjectManager (declaration)
  assert (declaration [1].type == "metadata")
  return manager
end
function Interest (declaration) return declaration end
function Constraint (declaration) return declaration end
{READINESS_BLOCK}
assert (default_metadata.subject == 0)
assert (default_metadata.key == "linx.target-cache-guard")
assert (default_metadata.spa_type == "Spa:String")
assert (default_metadata.value == "1")
local later_metadata = {{}}
function later_metadata:set (subject, key, spa_type, value)
  self.subject, self.key, self.spa_type, self.value = subject, key, spa_type, value
end
manager.callback (manager, later_metadata)
assert (later_metadata.key == "linx.target-cache-guard")
assert (later_metadata.value == "1")
print ("WirePlumber readiness manager: existing and future metadata passed")
"""
        _run_lua(fixture)


if __name__ == "__main__":
    unittest.main()
