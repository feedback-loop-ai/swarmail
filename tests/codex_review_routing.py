#!/usr/bin/env python3
"""Offline proofs through the recorded CLI, never a live provider.

Run: python3 tests/codex_review_routing.py --engine .forge/bin/brokkr --out <new-directory>
Controlled forge-driver/v1 participants exercise the production dispatcher,
aggregator, candidate fence and evaluator. Real recipe resolution is separate
from controlled dispatch: no claim about installed provider compatibility.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SAFE = dict(fixes_applied=False, spec_defect=False,
            has_security_residual=False, max_residual_severity="none")
DRIVER = r'''
import json,sys,pathlib,uuid,subprocess
model=sys.argv[-1]
config=json.loads(pathlib.Path('outcomes.json').read_text())
def send(**kw):
 print(json.dumps(dict(proto='forge-driver/v1',msg_id=str(uuid.uuid4()),**kw)),flush=True)
for line in sys.stdin:
 m=json.loads(line)
 if m['type']=='hello':send(type='capabilities',driver='controlled',version='1',supports=[])
 if m['type']!='start':continue
 p=pathlib.Path('.forge/calls.jsonl');p.parent.mkdir(exist_ok=True)
 with p.open('a') as f:f.write(json.dumps({'model':model,'seat':m['seat'],'result_path':m['input'].get('result_path')})+'\n')
 ids={k:m[k] for k in ['effect_id','attempt_id']}
 mode=config.get(model,config.get(m['seat'],'clean'))
 if mode=='fail':
  send(type='result',**ids,status='failed',error='controlled pre-session refusal');break
 send(type='accepted',**ids)
 if mode=='missing':break
 if mode=='cross-attempt':ids['attempt_id']='another-attempt'
 if mode=='cross-effect':ids['effect_id']='another-effect'
 if mode=='edit':
  pathlib.Path('candidate.txt').write_text('reviewer changed candidate\n')
  subprocess.run(['git','add','candidate.txt'],check=True,capture_output=True)
  subprocess.run(['git','-c','commit.gpgsign=false','commit','-qm','Change reviewed candidate'],check=True,capture_output=True)
 inputs={'fixes_applied':False,'spec_defect':False,'has_security_residual':False,'max_residual_severity':'none'}
 verdict='clean'
 if mode in ['hold','medium','high','security','scope','null','string','absent']:
  verdict='security-hold' if mode=='hold' else 'residual'
  inputs['max_residual_severity']='high' if mode=='high' else 'medium'
 if mode=='security':inputs['has_security_residual']=True
 if mode=='scope':inputs['spec_defect']=True
 if mode=='null':inputs['has_security_residual']=None
 if mode=='string':inputs['has_security_residual']='false'
 if mode=='absent':del inputs['has_security_residual']
 if mode=='bad-clean':inputs['spec_defect']='false'
 send(type='result',**ids,status='succeeded',result={'result':verdict,'inputs':inputs,'notes':'Controlled local result; no provider.'});break
'''
EXEC = '''import json,sys
from pathlib import Path
prompt=Path(sys.argv[1]).read_text()
paths=[s.strip() for s in prompt.splitlines() if s.strip().startswith('/') and s.strip().endswith('.json')]
Path(paths[-1]).write_text(json.dumps({'result':sys.argv[2],'inputs':{},'notes':'Local exec control.'}))
'''


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def call(args, cwd, log):
    result = subprocess.run(list(map(str, args)), cwd=cwd, capture_output=True,
                            text=True, timeout=60)
    log.with_suffix('.stdout').write_text(result.stdout)
    log.with_suffix('.stderr').write_text(result.stderr)
    return result


def resolved_matrix(engine, out):
    expected = {
        'self': {'review:correctness': ('dsh', ['qwen-max', 'glm-flash']),
                 'review:security': ('claude', ['opus', 'glm-flash']),
                 'review:guarantees': ('codex', ['astra', 'glm-flash']),
                 'review:tests': ('claude', ['fable', 'glm-flash']),
                 'intake': ('dsh', ['glm-flash']), 'implement': ('dsh', ['glm-flash'])},
        'codex': {'review:correctness': ('claude', ['fable']),
                  'review:security': ('dsh', ['flash']), 'review:tests': ('dsh', ['glm-flash'])},
    }
    five = {'correctness': ('codex', ['astra']), 'security': ('dsh', ['flash']),
            'tests': ('dsh', ['glm-flash']), 'guarantees': ('codex', ['astra']),
            'independent': ('claude', ['fable'])}
    expected['codex-careful'] = {'review:' + k: v for k, v in five.items()}
    expected['codex-fast'] = {'review:chore': ('dsh', ['flash']),
        'review:feature:correctness': ('dsh', ['qwen-max', 'sonnet']),
        'review:feature:security': ('dsh', ['flash']),
        **{'review:' + lane + ':' + k: v for lane in ['design', 'engine'] for k, v in five.items()}}
    expected['mixed-opus-terra'] = {'review:' + k: v for k, v in five.items()}
    expected['codex']['intake'] = ('codex', ['sol'])
    expected['codex-careful']['intake'] = ('codex', ['sol'])
    expected['mixed-opus-terra']['intake'] = ('codex', ['terra61'])
    expected['codex-fast']['triage'] = ('claude', ['opus'])
    wanted_triage = dict(agent='codex-triage', provider='claude', chain=['opus'],
        models=['opus'], concrete=['claude-opus-5-5'], efforts={'opus': 'medium'},
        resolution=[('opus', 'claude', 'ok')])
    manifests = {}
    for recipe in expected:
        result = call([engine, 'recipes', 'show', recipe], ROOT, out / (recipe + '-resolved'))
        assert result.returncode == 0, result.stderr
        data = json.loads(result.stdout)
        manifest = data['manifest']
        manifests[recipe] = manifest
        agents = manifest['agents']
        actual = {k: (v['provider'], v['chain']) for k, v in agents.items()
                  if k.startswith('review') or k in ['intake', 'triage']
                  or (recipe == 'self' and k == 'implement')}
        assert actual == expected[recipe], (recipe, actual)
        for key, agent in agents.items():
            if recipe != 'self' and key.startswith('implement'):
                work = {'codex-fast': ('codex', ['luna']),
                        'mixed-opus-terra': ('claude', ['opus'])}.get(recipe, ('codex', ['sol']))
                assert (agent['provider'], agent['chain']) == work, (recipe, key, agent)
        if recipe != 'self':
            keys = (['review:design:tests', 'review:engine:tests']
                    if recipe == 'codex-fast' else ['review:tests'])
            result = call([engine, 'agents', 'show', 'codex-review-tests'], ROOT,
                          out / (recipe + '-tests-office'))
            assert result.returncode == 0, result.stderr
            office = json.loads(result.stdout)
            adapter_path = ROOT / 'adapters/dsh.json'
            adapter = json.loads(adapter_path.read_text())
            wanted = dict(agent='codex-review-tests', provider='dsh',
                chain=['glm-flash'], model='glm-flash', chosen_index=0,
                models=['glm-flash'], efforts={}, effort=None,
                candidates=[dict(model='glm-flash', provider='dsh', effort=None)],
                concrete='spark-glm/GLM-5.3-Flash-EXL3', route='local',
                resolution=[('glm-flash', 'dsh', 'ok')],
                chosen={'index': 0, 'model': 'glm-flash', 'provider': 'dsh'},
                adapter_digest=next(a['adapter_digest'] for a in agents.values()
                                    if a['agent'] == 'codex-review-security'),
                charter_digest=office['charter_digest'])
            tests = {}
            for key, selected in agents.items():
                if not key.endswith(':tests'):
                    continue
                capability = manifest['capabilities']['sites'][key]
                assert capability['office'] == 'codex-review-tests'
                assert len(capability['candidates']) == 1
                pins = capability['candidates'][0]['model_pins']['read']
                assert pins == [{'id': adapter['models']['glm-flash'], 'route': 'spark-glm'}]
                candidates = (manifest['select']['review'][key.split(':')[1]]['panel']['tests']['candidates']
                              if recipe == 'codex-fast' else [selected])
                tests[key] = dict(agent=selected['agent'], provider=selected['provider'],
                    chain=selected['chain'], model=selected['model'],
                    chosen_index=selected['chosen_index'], effort=selected.get('effort'),
                    models=office['models'], efforts=office['efforts'], concrete=pins[0]['id'],
                    route=adapter['routes'][pins[0]['route']],
                    candidates=[dict(model=c['model'], provider=c['provider'], effort=c.get('effort'))
                                for c in candidates],
                    resolution=[(c['model'], c['provider'], c['status'])
                                for c in office['resolution']['chain']],
                    chosen=office['resolution']['chosen'],
                    adapter_digest=selected['adapter_digest'],
                    charter_digest=selected['charter_digest'])
            def assert_tests(matrix):
                assert matrix == {key: wanted for key in keys}, (recipe, matrix)
            assert_tests(tests)
            mutations = ['sonnet', 'provider', 'concrete', 'route', 'status',
                         'fallback', 'office-effort', 'compiled-effort', 'missing', 'replaced']
            for key in keys:
                for mutation in mutations:
                    bad = copy.deepcopy(tests)
                    row = bad[key]
                    if mutation == 'sonnet':
                        row.update(provider='claude', chain=['sonnet'], model='sonnet',
                                   models=['sonnet'], concrete='claude-sonnet-5-5')
                    elif mutation == 'provider':
                        row['provider'] = 'claude'
                    elif mutation == 'concrete':
                        row['concrete'] = 'dashscope/glm-5.3'
                    elif mutation == 'route':
                        row['route'] = 'uncontracted'
                    elif mutation == 'status':
                        row['resolution'] = [('glm-flash', 'dsh', 'blocked')]
                    elif mutation == 'fallback':
                        row['chain'].append('sonnet')
                    elif mutation == 'office-effort':
                        row['efforts'] = {'glm-flash': 'high'}
                    elif mutation == 'compiled-effort':
                        row['effort'] = 'high'
                        row['candidates'][0]['effort'] = 'high'
                    elif mutation == 'missing':
                        del bad[key]
                    else:
                        row['agent'] = 'review-tests'
                    try:
                        assert_tests(bad)
                    except AssertionError:
                        pass
                    else:
                        raise AssertionError(('tests mutant accepted', recipe, key, mutation))
            dump(out / (recipe + '-tests-matrix.json'),
                 dict(actual=tests, negative_controls=mutations, providers_called=False))
        # Resolve inherited efforts instead of inferring them from model aliases.
        for key in ['intake', *[k for k in agents if k.startswith('implement')]]:
            if recipe == 'self' or key not in agents:
                continue
            result = call([engine, 'agents', 'show', agents[key]['agent']], ROOT,
                          out / (recipe + '-' + key.replace(':', '-') + '-office'))
            assert result.returncode == 0, result.stderr
            office = json.loads(result.stdout)
            model = 'luna' if recipe == 'codex-fast' else 'sol'
            effort = ('medium' if key == 'intake' else 'high') if recipe == 'codex' else (
                'high' if key == 'intake' else 'xhigh')
            if recipe == 'mixed-opus-terra':
                model, effort = ('terra61' if key == 'intake' else 'opus'), 'high'
            assert office['models'] == [model] and office['efforts'] == {model: effort}
        if recipe == 'codex-fast':
            selected = agents['triage']
            result = call([engine, 'agents', 'show', selected['agent']], ROOT, out / 'triage-office')
            assert result.returncode == 0, result.stderr
            office = json.loads(result.stdout)
            adapter = json.loads((ROOT / 'adapters/claude.json').read_text())
            triage = dict(agent=selected['agent'], provider=selected['provider'],
                chain=selected['chain'], models=office['models'], efforts=office['efforts'],
                concrete=[adapter['models'][model] for model in office['models']],
                resolution=[(c['model'], c['provider'], c['status'])
                            for c in office['resolution']['chain']])
            def assert_triage(matrix):
                assert matrix == {'triage': wanted_triage}, matrix
            assert_triage({'triage': triage})
            # Each mutant must fail the comparison that accepted the real route.
            mutations = ['missing', 'provider', 'alias', 'model', 'effort', 'fallback']
            for mutation in mutations:
                bad = {'triage': copy.deepcopy(triage)}
                if mutation == 'missing':
                    del bad['triage']
                elif mutation == 'provider':
                    bad['triage']['provider'] = 'codex'
                elif mutation == 'alias':
                    bad['triage']['chain'] = ['sol']
                elif mutation == 'model':
                    bad['triage']['concrete'] = ['claude-sonnet-5-5']
                elif mutation == 'effort':
                    bad['triage']['efforts'] = {'opus': 'high'}
                else:
                    bad['triage']['chain'].append('sol')
                try:
                    assert_triage(bad)
                except AssertionError:
                    pass
                else:
                    raise AssertionError(('triage mutant accepted', mutation))
            dump(out / 'triage-matrix.json', dict(actual=triage, negative_controls=mutations,
                                                providers_called=False))
        # Negative controls run the same whole-matrix assertion used above.
        for key in [k for k in actual if k.startswith('review')]:
            for mutation in ['missing', 'pro', 'fallback', 'merged']:
                bad = copy.deepcopy(actual)
                if mutation == 'missing':
                    del bad[key]
                else:
                    bad[key] = ('dsh', ['deepseek-pro']) if mutation == 'pro' else ('claude', ['fable', 'sol']) if mutation == 'fallback' else ('codex', ['sol'])
                assert bad != expected[recipe], (recipe, key, mutation)
    base = manifests['self']
    shared_design = {k: v for k, v in base['agents'].items() if k.startswith('design:')}
    for recipe, manifest in manifests.items():
        assert {k: v for k, v in manifest['agents'].items()
                if k.startswith('design:')} == shared_design, recipe
        for key in ['design:check', 'verify', 'ship']:
            assert manifest['drivers'][key] == base['drivers'][key], (recipe, key)
            assert manifest['hands'][key] == base['hands'][key], (recipe, key)
            assert manifest['boundary'][key] == base['boundary'][key], (recipe, key)
    mixed_route(engine, out, manifests)
    print('resolved triage/intake, work/review matrices and negative controls passed', flush=True)


# Operator-directed mixed route (proposed 0090): office, provider, concrete pin,
# adapter route and office effort for each of its seven model seats.
MIXED_ROUTE = {
    'intake': ('mixed-terra-intake', 'codex', 'terra61', 'gpt-6.1-terra', None, 'high'),
    'implement': ('mixed-opus-implementer', 'claude', 'opus', 'claude-opus-5-5', None, 'high'),
    'review:correctness': ('codex-review-astra-correctness', 'codex', 'astra', 'gpt-6-astra', None, 'high'),
    'review:security': ('codex-review-security', 'dsh', 'flash', 'deepseek-flash',
                        ('deepseek-official', 'uncontracted'), 'high'),
    'review:tests': ('codex-review-tests', 'dsh', 'glm-flash', 'spark-glm/GLM-5.3-Flash-EXL3',
                     ('spark-glm', 'local'), None),
    'review:guarantees': ('codex-review-guarantees', 'codex', 'astra', 'gpt-6-astra', None, 'high'),
    'review:independent': ('codex-review-independent', 'claude', 'fable', 'claude-fable-5-1', None, 'high'),
}


def mixed_route(engine, out, manifests):
    mixed, careful = manifests['mixed-opus-terra'], manifests['codex-careful']
    adapters = {name: json.loads((ROOT / f'adapters/{name}.json').read_text())
                for name in ['codex', 'claude', 'dsh']}
    source = json.loads((ROOT / 'bundles/mixed-opus-terra/bundle.json').read_text())
    observed = {}
    for key in MIXED_ROUTE:
        selected = mixed['agents'].get(key)
        if selected is None:
            continue
        result = call([engine, 'agents', 'show', selected['agent']], ROOT,
                      out / ('mixed-' + key.replace(':', '-') + '-office'))
        assert result.returncode == 0, result.stderr
        office = json.loads(result.stdout)
        site = mixed['capabilities']['sites'][key]
        pins = [p for c in site['candidates'] for p in c['model_pins']['read']]
        observed[key] = dict(agent=selected['agent'], site_office=site['office'],
            provider=selected['provider'], chain=selected['chain'], model=selected['model'],
            chosen_index=selected['chosen_index'], skipped=selected['skipped'],
            effort=selected.get('effort'), models=office['models'], efforts=office['efforts'],
            resolution=[(c['model'], c['provider'], c['status']) for c in office['resolution']['chain']],
            chosen=office['resolution']['chosen'], concrete=[p['id'] for p in pins],
            route=[(p['route'], adapters[selected['provider']]['routes'][p['route']])
                   for p in pins if 'route' in p],
            charter=office['charter'], limits=office.get('limits'),
            hands=mixed['hands'].get(key), boundary=mixed['boundary'].get(key))
    observed['source'] = dict(extends=source['extends'], override=source['override'],
                              seats=sorted(source['seats']))
    observed['aliases'] = {a: adapters['codex']['models'][a] for a in ['terra', 'terra61']}

    def charter(agent):
        office = json.loads((ROOT / f'agents/{agent}.json').read_text())
        while 'charter' not in office:
            office = json.loads((ROOT / f"agents/{office['extends']}.json").read_text())
        return office['charter']
    wanted = {}
    for key, (agent, provider, model, concrete, route, effort) in MIXED_ROUTE.items():
        work = key in ['intake', 'implement']
        base = 'intake' if key == 'intake' else 'implementer'
        wanted[key] = dict(agent=agent, site_office=agent, provider=provider, chain=[model],
            model=model, chosen_index=0, skipped=[], effort=None, models=[model],
            efforts={model: effort} if effort else {},
            resolution=[(model, provider, 'ok')],
            chosen={'index': 0, 'model': model, 'provider': provider},
            concrete=[concrete], route=[route] if route else [], charter=charter(agent),
            limits=json.loads((ROOT / f'agents/{base}.json').read_text())['limits'] if work else None,
            hands=careful['hands'].get(key), boundary=careful['boundary'].get(key))
    wanted['source'] = dict(extends='codex-careful', seats=['implement', 'intake'],
        override={'bundle': ['description'], 'seats': ['intake', 'implement']})
    wanted['aliases'] = {'terra': 'gpt-5.6-terra', 'terra61': 'gpt-6.1-terra'}
    # Limits are the office layer; the recipe's own 7200s seat limit is source-only.
    assert source['seats']['implement']['limits'] == {'max_attempts': 3, 'timeout_seconds': 7200}
    assert wanted['implement']['limits']['timeout_seconds'] == 14400

    def assert_mixed(matrix):
        assert matrix == wanted, matrix
        # Everything outside the two work seats is careful's, field for field.
        for section in set(mixed) | set(careful):
            if section in ['agents', 'capabilities', 'files', 'bundle_name']:
                continue
            assert mixed[section] == careful[section], section
        for section in ['agents', 'sites']:
            a = mixed[section] if section == 'agents' else mixed['capabilities']['sites']
            b = careful[section] if section == 'agents' else careful['capabilities']['sites']
            assert set(a) == set(b), section
            assert sorted(k for k in a if a[k] != b[k]) == ['implement', 'intake'], section
        assert {k: v for k, v in mixed['capabilities'].items() if k != 'sites'} == \
            {k: v for k, v in careful['capabilities'].items() if k != 'sites'}
    assert_mixed(observed)
    mutations = ['model', 'provider', 'concrete', 'route', 'effort', 'compiled-effort',
                 'status', 'fallback', 'missing', 'replaced', 'hands', 'boundary']
    rejected = []
    for key in MIXED_ROUTE:
        for mutation in mutations:
            bad = copy.deepcopy(observed)
            row = bad[key]
            if mutation == 'model':
                row['concrete'] = ['claude-sonnet-5-5']
            elif mutation == 'provider':
                row['provider'] = 'codex' if row['provider'] != 'codex' else 'claude'
            elif mutation == 'concrete':
                row['concrete'] = ['dashscope/glm-5.3']
            elif mutation == 'route':
                row['route'] = [('spark-glm', 'local')] if key != 'review:tests' else [('dashscope', 'uncontracted')]
            elif mutation == 'effort':
                row['efforts'] = {row['model']: 'xhigh'}
            elif mutation == 'compiled-effort':
                row['effort'] = 'high'
            elif mutation == 'status':
                row['resolution'] = [(row['model'], row['provider'], 'blocked')]
            elif mutation == 'fallback':
                row['chain'].append('sonnet')
            elif mutation == 'missing':
                del bad[key]
            elif mutation == 'replaced':
                row['agent'] = 'review-correctness' if key.startswith('review') else 'implementer'
            elif mutation == 'hands':
                row['hands'] = {'kind': 'workspace', 'network': True, 'binds': []}
            else:
                row['boundary'] = 'harness'
            rejected.append(refuses(assert_mixed, bad, ('mixed', key, mutation)))
    for mutation in ['alias', 'terra61', 'override', 'seat', 'merged', 'extra-member']:
        bad = copy.deepcopy(observed)
        if mutation == 'alias':
            bad['aliases']['terra'] = 'gpt-6.1-terra'
        elif mutation == 'terra61':
            bad['aliases']['terra61'] = 'gpt-5.6-terra'
        elif mutation == 'override':
            bad['source']['override']['seats'].append('review')
        elif mutation == 'seat':
            bad['source']['seats'].append('review')
        elif mutation == 'merged':
            bad['review:guarantees'] = copy.deepcopy(bad['review:correctness'])
        else:
            bad['review:extra'] = copy.deepcopy(bad['review:tests'])
        rejected.append(refuses(assert_mixed, bad, ('mixed', mutation)))
    dump(out / 'mixed-route-matrix.json', dict(actual=observed, rejected=rejected,
         presence='unknown: resolution is not a provider invocation', providers_called=False))


def refuses(comparator, bad, label):
    try:
        comparator(bad)
    except AssertionError:
        return label
    raise AssertionError(('mutant accepted',) + label)


def fixture(engine, out, name, roles, outcomes, fallback=False, single=False):
    case = out / name
    case.mkdir()
    (case / 'driver.py').write_text(DRIVER)
    dump(case / 'outcomes.json', outcomes)
    (case / 'candidate.txt').write_text('verified candidate\n')
    (case / '.gitignore').write_text('.forge/\n')
    (case / 'house.txt').write_text('Local controlled proof, no delivery.\n')
    dump(case / 'realms.json', {'schema': 'forge.realms/v4', 'realms': [
        dict(name='probe', path='.', default_branch='main', house='house.txt', boundary='harness')],
        'journal': '.forge/probe.db'})
    # Existing policies are used verbatim for review rules, including all hard stops.
    policy = json.loads((ROOT / 'bundles/self/policy.json').read_text())
    review_rules = [r for r in policy['rules'] if r['from'] == 'review']
    table = dict(schema='forge.phase-machine/v1', phases=['prepare','verify','review','ship','done','stop'],
                 initial='prepare', terminal=['done','stop'], shippable_from=['review'], rules=[
        dict(id='PREPARE', **{'from':'prepare'}, result='complete', next='verify', reason='Fixture setup.'),
        dict(id='VERIFIED', **{'from':'verify'}, result='verified', next='review', reason='Candidate witness.'),
        *review_rules,
        dict(id='SHIPPED', **{'from':'ship'}, result='shipped', next='done', reason='Local terminal only.')])
    # Return targets exist but are terminal in this fixture: policy full traces are
    # separately covered by the source-linked policy sweep, not guessed here.
    for rule in table['rules']:
        if rule['next'] not in table['phases']:
            rule['next'] = 'stop'
    bundle_dir = case / 'recipes/probe'
    bundle_dir.mkdir(parents=True)
    (bundle_dir / 'exec.py').write_text(EXEC)
    exec_adapter = json.loads((ROOT/'adapters/exec.json').read_text())
    dump(case/'adapters/exec.json', exec_adapter)
    models = {role: role for role in roles}
    if fallback:
        models.update({'qwen-max':'dashscope/qwen3.8-max','sonnet':'claude-sonnet-5-5'})
    adapter = dict(provider='controlled', trust_tier='trusted', egress='uncontracted', binary='python3',
        driver=['python3', str(case/'driver.py')], models=models, judges=list(models),
        model_flag='--model', efforts=[], effort_flag='unsupported', tool_permissions='unsupported',
        mcp='unsupported', hands={'workspace':[]})
    dump(case/'adapters/controlled.json', adapter)
    panel = {}
    for role in roles:
        dump(case/f'agents/{role}.json', {'description':'Controlled review member', 'charter':'review.md',
            'models':['qwen-max','sonnet'] if fallback and role=='correctness' else [role],
            **({'inputs':list(SAFE)} if single else {})})
        panel[role] = {'class':'gate','agent':role}
    (case/'agents/review.md').write_text('Read-only controlled reviewer.\n')
    def exec_seat(word):
        return {'class':'gate','driver':{'command':['{brokkr}','driver','exec','--','python3',
                './exec.py','{prompt_file}',word]}, 'hands':{'kind':'workspace','network':False,'binds':[]},
                'results':[word], 'limits':{'max_attempts':1,'timeout_seconds':10}}
    review = dict(results=['clean','residual','security-hold'], inputs=list(SAFE),
                  limits={'max_attempts':3,'timeout_seconds':10})
    review.update(panel[roles[0]] if single else {'aggregate':'review-panel','panel':panel})
    if single:
        del review['inputs']
    bundle = dict(name='probe', policy='policy.json', protected_phase='review', seats={
        'prepare':exec_seat('complete'), 'verify':exec_seat('verified'), 'review':review,
        'ship':exec_seat('shipped')})
    dump(bundle_dir/'bundle.json', bundle)
    dump(bundle_dir/'policy.json', table)
    for args in [['git','init','-qb','main'], ['git','config','user.email','probe@invalid'],
                 ['git','config','user.name','Routing proof'], ['git','add','.'],
                 ['git','-c','commit.gpgsign=false','commit','-qm','Prepare controlled review']]:
        subprocess.run(args,cwd=case,check=True,capture_output=True)
    run = call([engine,'run','--bundle','recipes/probe','--db','.forge/probe.db',
                '--feature','Controlled offline review proof; no publication','--no-view'],case,out/(name+'-run'))
    view = call([engine,'inspect','--db','.forge/probe.db','--run','latest','--json'],case,out/(name+'-inspect'))
    assert view.returncode == 0, (name,run.stderr,view.stderr)
    data = json.loads(view.stdout)
    journal = [{'type':e['event_type'],'payload':json.loads(e['payload_json'])} for e in data['journal']]
    calls_path = case/'.forge/calls.jsonl'
    calls = [json.loads(line) for line in calls_path.read_text().splitlines()] if calls_path.exists() else []
    return dict(case=name,status=data['summary']['status'],calls=calls,journal=journal)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine',required=True,type=Path)
    parser.add_argument('--out',required=True,type=Path)
    args=parser.parse_args()
    engine=args.engine.resolve();out=args.out.resolve();out.mkdir(parents=True,exist_ok=False)
    record=json.loads((ROOT/'bundles/self/tooling.json').read_text())['brokkr']
    digest=hashlib.sha256(engine.read_bytes()).hexdigest()
    assert digest==record['binary_sha256'], 'Refuse an unrecorded engine'
    resolved_matrix(engine,out)
    rows=[]
    def check(name,roles,outcomes,want,fallback=False,single=False):
        row=fixture(engine,out,name,roles,outcomes,fallback,single)
        assert (row['status']=='completed')==want,(name,row['status'])
        rows.append(row)
        print(name,row['status'],flush=True)
        return row
    for mode,want in [('fail',True),('clean',True),('high',False),('bad-clean',False),('null',False),('string',False),('absent',False)]:
        row=check('qwen-'+mode,['correctness','security'],{'dashscope/qwen3.8-max':mode},want,True)
        models=[c['model'] for c in row['calls'] if c['seat']=='review:correctness']
        assert models==(['dashscope/qwen3.8-max','claude-sonnet-5-5'] if mode=='fail' else ['dashscope/qwen3.8-max']),models
    row=check('exhausted',['correctness','security'],{'dashscope/qwen3.8-max':'fail','claude-sonnet-5-5':'fail'},False,True)
    assert [c['model'] for c in row['calls'] if c['seat']=='review:correctness']==['dashscope/qwen3.8-max','claude-sonnet-5-5','claude-sonnet-5-5']
    row=check('security-failure',['correctness','security'],{'security':'fail'},False,True)
    assert all(c['model']=='security' for c in row['calls'] if c['seat']=='review:security')
    for roles in [['security'],['correctness','security'],['correctness','security','tests'],
                  ['correctness','security','tests','guarantees','independent']]:
        n=len(roles);single=n==1
        check(f'{n}-clean',roles,{},True,single=single)
        for mode in ['hold','high','security','null','string','absent','scope','missing','cross-attempt','cross-effect','edit']:
            subject='security' if mode=='hold' else roles[-1]
            row=check(f'{n}-{mode}',roles,{subject:mode},False,single=single)
            if mode=='missing':
                assert not any(e['type']=='effect/succeeded' and e['payload'].get('result',{}).get('result')=='clean' for e in row['journal'])
        if n==5:
            row=check('independent-medium',roles,{'independent':'medium'},True)
            assert any(e['type']=='transition/decided' and e['payload']['from']=='review' and e['payload']['result']=='residual' for e in row['journal'])
    dump(out/'receipt.json',dict(engine_sha256=digest,providers_called=False,
        limitations=['Controlled protocol participants do not prove live-provider compatibility.',
                     'Return destinations are terminal in dispatch fixtures; full policy traces require the core evaluator.'],rows=rows))


if __name__=='__main__':
    main()
