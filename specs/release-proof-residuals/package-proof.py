#!/usr/bin/env python3
"""Offline Cargo membership/archive proof using only synthetic cache decoys.

Run from the repository root: python3 specs/release-proof-residuals/package-proof.py
Evidence remains under .forge/proof186/package; no registry publication occurs.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path.cwd()
EVIDENCE = ROOT / '.forge/proof186/package'
EVIDENCE.mkdir(parents=True, exist_ok=True)
ENV = {**os.environ, 'CARGO_NET_OFFLINE': 'true', 'RUSTUP_OFFLINE': '1'}
RECORDS = []


def cargo(label, directory, *args):
    command = ['cargo', 'package', '--locked', '--offline', *(['--allow-dirty'] if directory != ROOT else []), *args]
    result = subprocess.run(command, cwd=directory, env=ENV, capture_output=True, text=True)
    (EVIDENCE / (label + '.stdout')).write_text(result.stdout)
    (EVIDENCE / (label + '.stderr')).write_text(result.stderr)
    RECORDS.append({'case': label, 'command': command, 'exit': result.returncode})
    assert result.returncode == 0, (label, result.stderr)
    return set(result.stdout.splitlines())


def membership(label, members, required):
    missing = required - members
    leaked = sorted(p for p in members if p.startswith('.cargo-home/'))
    assert not missing and not leaked, (label, 'missing', sorted(missing), 'leaked', leaked)


patterns = ['src/**', 'tests/**', 'benches/**', 'assets/**',
            'scripts/data/html-entities.json', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE']
manifest = (ROOT / 'Cargo.toml').read_text()
metadata = tomllib.loads(manifest)['package']
assert metadata['include'] == ['/' + pattern for pattern in patterns]
required = {'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', 'README.md',
            'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'scripts/data/html-entities.json'}
for family in ['src', 'tests', 'benches', 'assets']:
    required.update(str(p.relative_to(ROOT)) for p in (ROOT / family).rglob('*') if p.is_file())
current = cargo('candidate', ROOT, '--list')
membership('candidate', current, required)

with tempfile.TemporaryDirectory(prefix='swarmail-package-proof-') as scratch:
    fixture = Path(scratch)
    for name in ['Cargo.toml', 'Cargo.lock', 'README.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE']:
        shutil.copy2(ROOT / name, fixture / name)
    for family in ['src', 'tests', 'benches', 'assets']:
        shutil.copytree(ROOT / family, fixture / family)
    data = fixture / 'scripts/data'
    data.mkdir(parents=True)
    shutil.copy2(ROOT / 'scripts/data/html-entities.json', data)
    # No nested Cargo.toml can hide matching cache files. Never copy a real cache.
    decoys = ['src/decoy.rs', 'tests/decoy.rs', 'benches/decoy.rs', 'assets/decoy.svg',
              'scripts/data/html-entities.json', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE']
    for name in decoys:
        path = fixture / '.cargo-home/synthetic' / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('SYNTHETIC PACKAGE DECOY: ' + name + '\n')
    baseline = cargo('fixture-anchored', fixture, '--list')
    membership('fixture-anchored', baseline, required)
    delta = sorted(current ^ baseline)
    # Cargo adds Git provenance only when packaging from a Git repository.
    assert set(delta) <= {'.cargo_vcs_info.json'}, delta
    RECORDS.append({'candidate_fixture_delta': delta, 'explanation': 'automatic Git provenance only'})
    before = manifest
    for pattern in patterns[:5]:
        before = before.replace('"/' + pattern + '"', '"' + pattern + '"')
    (fixture / 'Cargo.toml').write_text(before)
    inherited = cargo('fixture-before', fixture, '--list')
    assert inherited == baseline, sorted(inherited ^ baseline)
    RECORDS.append({'before_after_delta': [], 'explanation': 'slash-containing patterns already root-relative'})
    for pattern in patterns:
        (fixture / 'Cargo.toml').write_text(manifest.replace('"/' + pattern + '"', '"' + pattern + '"'))
        members = cargo('unanchor-' + pattern.replace('/', '_').replace('*', 'all'), fixture, '--list')
        if '/' in pattern:
            assert members == baseline, (pattern, sorted(members ^ baseline))
            RECORDS.append({'pattern': pattern, 'result': 'measured membership equivalence'})
        else:
            expected = '.cargo-home/synthetic/' + pattern
            assert members - baseline == {expected} and not baseline - members, (pattern, sorted(members ^ baseline))
            try:
                membership(pattern, members, required)
            except AssertionError as error:
                assert expected in str(error)
                RECORDS.append({'pattern': pattern, 'result': 'owning assertion red', 'diagnostic': str(error)})
            else:
                raise AssertionError('unanchored basename unexpectedly admitted: ' + pattern)
        (fixture / 'Cargo.toml').write_text(manifest)
        restored = cargo('restored-' + pattern.replace('/', '_').replace('*', 'all'), fixture, '--list')
        assert restored == baseline
        membership('restored-' + pattern, restored, required)
    # Build the actual archive without compiling it or publishing. Full build/test
    # gates are separate; the package boundary is the proof owned here.
    cargo('archive', fixture, '--no-verify')
    archive = fixture / 'target/package' / (metadata['name'] + '-' + metadata['version'] + '.crate')
    with tarfile.open(archive) as package:
        prefix = metadata['name'] + '-' + metadata['version'] + '/'
        packaged = set(n.removeprefix(prefix) for n in package.getnames())
        membership('archive', packaged, required)
        hashes = {}
        for name in ['LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'scripts/data/html-entities.json', 'Cargo.lock']:
            actual = package.extractfile(prefix + name).read()
            assert actual == (ROOT / name).read_bytes(), name
            hashes[name] = hashlib.sha256(actual).hexdigest()
        assert package.extractfile(prefix + 'Cargo.toml.orig').read() == manifest.encode()
        normalized = tomllib.loads(package.extractfile(prefix + 'Cargo.toml').read().decode())['package']
        for field in ['name', 'version', 'license', 'repository', 'readme', 'description', 'keywords', 'categories', 'edition']:
            assert normalized[field] == metadata[field], field
        RECORDS.append({'archive_bytes_sha256': hashes, 'metadata': 'preserved'})
    shutil.copy2(archive, EVIDENCE / archive.name)

(EVIDENCE / 'receipt.json').write_text(json.dumps(RECORDS, indent=2) + '\n')
print('Cargo package proof: root members and archive bytes preserved; three basename controls red/restored; five slash patterns equivalent; synthetic cache excluded.')
