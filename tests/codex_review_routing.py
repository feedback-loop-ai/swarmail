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
        'codex': {'review:correctness': ('claude', ['fable']),
                  'review:security': ('dsh', ['flash']), 'review:tests': ('claude', ['sonnet'])},
    }
    five = {'correctness': ('codex', ['astra']), 'security': ('dsh', ['flash']),
            'tests': ('claude', ['sonnet']), 'guarantees': ('codex', ['astra']),
            'independent': ('claude', ['fable'])}
    expected['codex-careful'] = {'review:' + k: v for k, v in five.items()}
    expected['codex-fast'] = {'review:chore': ('dsh', ['flash']),
        'review:feature:correctness': ('dsh', ['qwen-max', 'sonnet']),
        'review:feature:security': ('dsh', ['flash']),
        **{'review:' + lane + ':' + k: v for lane in ['design', 'engine'] for k, v in five.items()}}
    for recipe in ['self', *expected]:
        result = call([engine, 'recipes', 'show', recipe], ROOT, out / (recipe + '-resolved'))
        assert result.returncode == 0, result.stderr
        data = json.loads(result.stdout)
        if recipe == 'self':
            continue
        agents = data['manifest']['agents']
        actual = {k: (v['provider'], v['chain']) for k, v in agents.items() if k.startswith('review')}
        assert actual == expected[recipe], (recipe, actual)
        for key, agent in agents.items():
            if key.startswith('implement'):
                chain = ['luna'] if recipe == 'codex-fast' else ['sol']
                assert agent['provider'] == 'codex' and agent['chain'] == chain
        # Negative controls run the same whole-matrix assertion used above.
        for key in actual:
            for mutation in ['missing', 'pro', 'fallback', 'merged']:
                bad = copy.deepcopy(actual)
                if mutation == 'missing':
                    del bad[key]
                else:
                    bad[key] = ('dsh', ['deepseek-pro']) if mutation == 'pro' else ('claude', ['fable', 'sol']) if mutation == 'fallback' else ('codex', ['sol'])
                assert bad != expected[recipe], (recipe, key, mutation)


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
