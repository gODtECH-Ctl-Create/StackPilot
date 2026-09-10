from pathlib import Path

path = Path('src/fix.rs')
text = path.read_text()
old = '        None => ("Generic", "None", repository_name(root)),\n'
new = '        None => ("Generic", "None", env_value(&repository_name(root))),\n'
if old not in text:
    raise SystemExit('Terraform fallback project name pattern not found')
path.write_text(text.replace(old, new, 1))
