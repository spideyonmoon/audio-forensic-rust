"""Transcribe the pinned scalar audit label functions, preserving exact wording.
No product headlines use these labels. --check never rewrites generated source.
"""
import ast,inspect,sys,textwrap,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'reference/audio-forensic'))
import audio_forensic as py
NAMES=['variance','sharpness','hf_ratio','banding','nf','side','entropy','bound','phase_entropy','sparsity','ultra_corr','mdct']
def expr(n):
 if isinstance(n,ast.UnaryOp):return '-'+expr(n.operand)
 if isinstance(n,ast.Constant):return json.dumps(n.value,ensure_ascii=False) if isinstance(n.value,str) else str(float(n.value))
 if isinstance(n,ast.Name):return n.id
 if isinstance(n,ast.Attribute):assert ast.unparse(n)=='self.nyquist';return 'ny'
 if isinstance(n,ast.BinOp):return f'({expr(n.left)} * {expr(n.right)})'
 if isinstance(n,ast.Compare):
  assert len(n.ops)==1
  op={ast.Lt:'<',ast.Gt:'>',ast.LtE:'<=',ast.GtE:'>=',ast.Eq:'=='}[type(n.ops[0])]
  return f'{expr(n.left)} {op} {expr(n.comparators[0])}'
 if isinstance(n,ast.IfExp):return f'if {expr(n.test)} {{ {expr(n.body)} }} else {{ {expr(n.orelse)} }}'
 raise ValueError(ast.dump(n))
def statements(nodes):
 out=''
 for n in nodes:
  if isinstance(n,ast.Return):out+='return '+expr(n.value)+';\n'
  elif isinstance(n,ast.If):
   out+='if '+expr(n.test)+' {\n'+statements(n.body)+'}\n'
   out+=statements(n.orelse)
  else:raise ValueError(ast.dump(n))
 return out
def generate():
 s='#![allow(clippy::needless_return)] // Preserve the pinned audit function structure.\n// Generated from pinned Python audit labels; see generate_reference_labels.py.\n'
 for name in NAMES:
  node=ast.parse(textwrap.dedent(inspect.getsource(getattr(py.SpectralEngine,'_interp_'+name)))).body[0]
  args=[a.arg for a in node.args.args]
  args=['ny' if a=='self' else a for a in args]
  s+='pub(super) fn '+name+'('+', '.join(a+(': bool' if a=='legit_cutoff' else ': f64') for a in args)+") -> &'static str {\n"+statements(node.body)+'}\n'
 return s
if __name__=='__main__':
 p=ROOT/'src/reference_labels.rs';s=generate()
 if '--check' in sys.argv:
  # Rustfmt whitespace is irrelevant to generated tokens.
  assert ''.join(p.read_text(encoding='utf-8').split())==''.join(s.split())
 else:p.open('x',encoding='utf-8').write(s)
 print('PASS pinned audit label transcription')
