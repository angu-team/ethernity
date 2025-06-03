import subprocess
import re
import os
import time
import shutil
import argparse
import json
from ollama import Client

MAX_CONTEXT_LINES = 100
MAX_FILE_SIZE_KB = 200
BACKUP_DIR = "ai_fixer_backups"
ERROR_LOG = "ai_fixer_errors.log"


class VPSAIFixer:

    def __init__(self, vps_address, model='llama3:70b'):
        self.client = Client(host=vps_address)
        self.model = model
        self.setup_backup_system()
        self.error_log = open(ERROR_LOG, "a")
        print(f"🚀 Modo VPS ativado | Modelo: {model} | Servidor: {vps_address}")

    def setup_backup_system(self):
        if not os.path.exists(BACKUP_DIR):
            os.makedirs(BACKUP_DIR)
        self.current_backup = os.path.join(BACKUP_DIR, f"backup_{int(time.time())}")
        os.makedirs(self.current_backup, exist_ok=True)

    def backup_file(self, file_path):
        try:
            if os.path.getsize(file_path) < (200 * 1024):
                shutil.copy2(file_path, os.path.join(self.current_backup, os.path.basename(file_path)))
        except Exception as e:
            print(f"⚠️ Falha no backup: {e}")

    def light_compile(self):
        try:
            result = subprocess.run(
                ['cargo', 'build', '--message-format=json'],
                capture_output=True,
                text=True
            )
            full_output = result.stdout + result.stderr
            if full_output.strip():
                self.error_log.write(f"COMPILATION OUTPUT:\n{full_output}\n\n")
            return full_output
        except Exception as e:
            print(f"🔧 Erro na compilação: {e}")
            return ""

    def extract_errors(self, log):
        errors = []
        for line in log.splitlines():
            try:
                message = json.loads(line)
                if message.get("reason") == "compiler-message" and message.get("message"):
                    msg_obj = message["message"]
                    if msg_obj.get("level") == "error":
                        text = msg_obj.get("rendered", msg_obj.get("message", ""))
                        spans = msg_obj.get("spans", [])
                        file_path = None
                        if spans:
                            first_span = spans[0]
                            file_path = first_span.get("file_name")
                        errors.append({
                            "text": text,
                            "file": file_path
                        })
            except json.JSONDecodeError:
                # Se a linha não é JSON, verifica se parece um erro
                if "error:" in line or "error[" in line:
                    errors.append({"text": line, "file": None})
            # Limite de erros
            if len(errors) >= 3:
                break
        return errors

    def find_error_source(self, error_line):
        patterns = [
            r'--> (src/.+\.rs)',
            r'at (src/.+\.rs:\d+)',
            r'in (crates/.+\.rs)',
            r'--> (.+\.rs):\d+:\d+',
            r'\/rustc\/[^\/]+\/(.+)\.rs:\d+',
            r'^ *\|\n *\d+ \| (.*\.rs)',
            r'error\[E\d+\] in (.+\.rs)'
        ]
        for pattern in patterns:
            match = re.search(pattern, error_line)
            if match:
                candidate = match.group(1)
                # Tentar caminhos relativos e absolutos
                for path in [candidate, os.path.join("src", candidate), os.path.join("crates", candidate)]:
                    if os.path.exists(path):
                        return path
        return None

    def smart_file_read(self, file_path):
        try:
            if os.path.getsize(file_path) > (MAX_FILE_SIZE_KB * 1024):
                print(f"📁 Arquivo grande, lendo amostra: {file_path}")
                with open(file_path, 'r', encoding='utf-8') as f:
                    lines = []
                    for i, line in enumerate(f):
                        if i < MAX_CONTEXT_LINES:
                            lines.append(line)
                        else:
                            break
                    return ''.join(lines)
            else:
                with open(file_path, 'r', encoding='utf-8') as f:
                    return f.read()
        except Exception as e:
            print(f"📁 Erro de leitura: {e}")
            return ""

    def generate_efficient_prompt(self, file_path, code_snippet, error):
        return f"""
Corrija este erro Rust:

## Arquivo

{os.path.basename(file_path)}

## Erro

{error}

## Código

```rust
{code_snippet}
```

## Instruções

1. Retorne APENAS o código corrigido entre ```rust
2. Mantenha a estrutura e formatação original
3. Não adicione explicações ou comentários
4. Se não houver correção necessária, retorne o código original

Código corrigido:
"""

    def fix_with_vps(self, file_path, error):
        if not file_path or not os.path.exists(file_path):
            print(f"⚠️  Arquivo não encontrado: {file_path}")
            return False
        print(f"🔧 Tentando corrigir: {os.path.basename(file_path)}")
        self.backup_file(file_path)
        code_snippet = self.smart_file_read(file_path)
        if not code_snippet:
            return False

        prompt = self.generate_efficient_prompt(file_path, code_snippet, error)
        try:
            response = self.client.chat(
                model=self.model,
                messages=[{'role': 'user', 'content': prompt}],
                options={'num_ctx': 4096}
            )
            fixed_code = response['message']['content'].strip()
            # Extração robusta de código
            if "```rust" in fixed_code:
                fixed_code = fixed_code.split("```rust")[1].split("```")[0].strip()
            elif "```" in fixed_code:
                fixed_code = fixed_code.split("```")[1].split("```")[0].strip()
            if fixed_code.startswith("rust\n"):
                fixed_code = fixed_code[5:]
            # Verificação de alterações
            if fixed_code and fixed_code != code_snippet:
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write(fixed_code)
                print(f"✅ Código atualizado")
                return True
            else:
                print("⚠️ Sem alterações no código")
                return False
        except Exception as e:
            print(f"🌐 ERRO na comunicação com VPS: {e}")
            return False

    def run(self):
        print("⚡ Iniciando correção automática")
        for attempt in range(1, 6):
            print(f"\n🔄 Tentativa {attempt}/5")
            build_log = self.light_compile()
            # Verificar se não há saída de erro (não confiável apenas com string)
            # Em vez disso, confiamos na lista de erros extraída
            errors = self.extract_errors(build_log)
            if not errors:
                print("🎉 Compilação bem-sucedida!")
                return True
            print(f"⚠️  Erros encontrados: {len(errors)}")
            self.error_log.write(f"ATTEMPT {attempt} ERRORS:\n{json.dumps(errors, indent=2)}\n\n")
            # Tentar corrigir cada erro sequencialmente
            for error in errors:
                error_text = error["text"]
                file_path = error.get("file")
                print(f"\n🔍 Analisando erro: {error_text[:120]}...")
                # Se não temos caminho do JSON, tentamos extrair por regex
                if not file_path:
                    file_path = self.find_error_source(error_text)
                # Se ainda não temos, tentamos alguns caminhos comuns
                if file_path:
                    # Verificar existência do arquivo
                    if not os.path.exists(file_path):
                        # Tentar caminhos alternativos
                        base_name = os.path.basename(file_path)
                        possible_paths = [
                            file_path,
                            os.path.join("src", base_name),
                            os.path.join("crates", base_name),
                            os.path.join("src", file_path),
                            os.path.join("crates", file_path),
                            os.path.join("crates", os.path.basename(file_path))
                        ]
                        found = False
                        for path in possible_paths:
                            if os.path.exists(path):
                                file_path = path
                                found = True
                                break
                        if not found:
                            print(f"⚠️  Arquivo não encontrado: {file_path}")
                            continue
                    print(f"📄 Arquivo identificado: {file_path}")
                    if self.fix_with_vps(file_path, error_text):
                        break  # Pausar após uma correção para recompilar
            time.sleep(1.5)  # Pequeno delay entre tentativas

        print("\n⚠️  Limite de tentativas atingido")
        print(f"💡 Dica: Verifique erros detalhados em {ERROR_LOG}")
        print("💡 Considere: 1) Verificar manualmente 2) Usar modelo diferente 3) Expandir contexto")
        return False


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='AI Fixer com trabalho na VPS')
    parser.add_argument('--vps', required=True, help='Endereço da VPS (ex: http://192.168.1.100:11434)')
    parser.add_argument('--model', default='llama3:70b', help='Modelo grande na VPS')
    args = parser.parse_args()

    fixer = VPSAIFixer(vps_address=args.vps, model=args.model)
    fixer.run()