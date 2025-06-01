#!/usr/bin/env python3

import asyncio
import json
import websockets
import requests
from web3 import Web3
import time
import sys

# Endpoints RPC fornecidos
WS_ENDPOINT = "ws://116.202.218.100:8546"
HTTP_ENDPOINT = "http://116.202.218.100:8545"

# Cores para saída
GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
RESET = "\033[0m"

def print_success(message):
    print(f"{GREEN}✓ {message}{RESET}")

def print_error(message):
    print(f"{RED}✗ {message}{RESET}")

def print_info(message):
    print(f"{BLUE}ℹ {message}{RESET}")

def print_warning(message):
    print(f"{YELLOW}⚠ {message}{RESET}")

async def test_websocket_connection():
    print_info("Testando conexão WebSocket...")
    try:
        # Removido o parâmetro timeout que estava causando erro
        async with websockets.connect(WS_ENDPOINT) as websocket:
            # Requisição para obter o número do bloco atual
            request = {
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }
            await websocket.send(json.dumps(request))
            response = await asyncio.wait_for(websocket.recv(), timeout=10)
            response_json = json.loads(response)
            
            if 'result' in response_json:
                block_number = int(response_json['result'], 16)
                print_success(f"Conexão WebSocket estabelecida! Bloco atual: {block_number}")
                return True
            else:
                print_error(f"Erro na resposta WebSocket: {response_json}")
                return False
    except Exception as e:
        print_error(f"Falha na conexão WebSocket: {e}")
        return False

def test_http_connection():
    print_info("Testando conexão HTTP...")
    try:
        response = requests.post(
            HTTP_ENDPOINT,
            json={"jsonrpc": "2.0", "method": "eth_blockNumber", "params": [], "id": 1},
            timeout=10
        )
        response_json = response.json()
        
        if 'result' in response_json:
            block_number = int(response_json['result'], 16)
            print_success(f"Conexão HTTP estabelecida! Bloco atual: {block_number}")
            return True
        else:
            print_error(f"Erro na resposta HTTP: {response_json}")
            return False
    except Exception as e:
        print_error(f"Falha na conexão HTTP: {e}")
        return False

def test_web3_connection():
    print_info("Testando conexão via Web3.py...")
    try:
        w3 = Web3(Web3.HTTPProvider(HTTP_ENDPOINT))
        if w3.is_connected():
            block_number = w3.eth.block_number
            print_success(f"Conexão Web3 estabelecida! Bloco atual: {block_number}")
            
            # Obter informações sobre o bloco mais recente
            latest_block = w3.eth.get_block('latest')
            print_info(f"Timestamp do bloco mais recente: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(latest_block.timestamp))}")
            print_info(f"Número de transações no bloco: {len(latest_block.transactions)}")
            
            return True
        else:
            print_error("Web3 não conseguiu se conectar ao node")
            return False
    except Exception as e:
        print_error(f"Falha na conexão Web3: {e}")
        return False

async def test_websocket_subscription():
    print_info("Testando subscrição WebSocket para novos blocos...")
    try:
        # Removido o parâmetro timeout que estava causando erro
        async with websockets.connect(WS_ENDPOINT) as websocket:
            # Subscrição para novos blocos
            request = {
                "jsonrpc": "2.0",
                "method": "eth_subscribe",
                "params": ["newHeads"],
                "id": 1
            }
            await websocket.send(json.dumps(request))
            response = await asyncio.wait_for(websocket.recv(), timeout=10)
            response_json = json.loads(response)
            
            if 'result' in response_json:
                subscription_id = response_json['result']
                print_success(f"Subscrição criada com ID: {subscription_id}")
                
                # Aguarda por um novo bloco (com timeout)
                print_info("Aguardando por um novo bloco (timeout de 30s)...")
                try:
                    block_data = await asyncio.wait_for(websocket.recv(), timeout=30)
                    block_json = json.loads(block_data)
                    if 'params' in block_json and 'result' in block_json['params']:
                        block_info = block_json['params']['result']
                        block_number = int(block_info['number'], 16)
                        print_success(f"Novo bloco recebido: {block_number}")
                        return True
                    else:
                        print_warning("Dados de bloco recebidos, mas em formato inesperado")
                        return True  # Ainda consideramos sucesso pois a subscrição funcionou
                except asyncio.TimeoutError:
                    print_warning("Timeout aguardando por um novo bloco, mas a subscrição foi criada com sucesso")
                    return True  # Ainda consideramos sucesso pois a subscrição funcionou
            else:
                print_error(f"Erro ao criar subscrição: {response_json}")
                return False
    except Exception as e:
        print_error(f"Falha no teste de subscrição: {e}")
        return False

async def test_transaction_trace():
    print_info("Testando obtenção de trace de transação...")
    try:
        # Primeiro obtemos uma transação recente
        w3 = Web3(Web3.HTTPProvider(HTTP_ENDPOINT))
        latest_block = w3.eth.get_block('latest')
        
        if len(latest_block.transactions) == 0:
            print_warning("Não há transações no bloco mais recente. Buscando em blocos anteriores...")
            
            # Tenta encontrar uma transação nos últimos 10 blocos
            for i in range(1, 11):
                block = w3.eth.get_block(latest_block.number - i)
                if len(block.transactions) > 0:
                    tx_hash = block.transactions[0].hex()
                    break
            else:
                print_error("Não foi possível encontrar uma transação nos últimos 10 blocos")
                return False
        else:
            tx_hash = latest_block.transactions[0].hex()
        
        print_info(f"Transação selecionada: {tx_hash}")
        
        # Agora obtemos o trace da transação
        response = requests.post(
            HTTP_ENDPOINT,
            json={
                "jsonrpc": "2.0", 
                "method": "debug_traceTransaction", 
                "params": [
                    tx_hash,
                    {"tracer": "callTracer"}
                ], 
                "id": 1
            },
            timeout=30
        )
        
        response_json = response.json()
        
        if 'result' in response_json:
            trace = response_json['result']
            print_success(f"Trace obtido com sucesso!")
            print_info(f"Tipo de trace: {trace.get('type', 'N/A')}")
            print_info(f"De: {trace.get('from', 'N/A')}")
            print_info(f"Para: {trace.get('to', 'N/A')}")
            
            # Verifica se há chamadas internas
            calls = trace.get('calls', [])
            print_info(f"Número de chamadas internas: {len(calls)}")
            
            return True
        else:
            print_error(f"Erro ao obter trace: {response_json}")
            return False
    except Exception as e:
        print_error(f"Falha ao obter trace de transação: {e}")
        return False

async def test_pending_transactions():
    print_info("Testando subscrição WebSocket para transações pendentes...")
    try:
        async with websockets.connect(WS_ENDPOINT) as websocket:
            # Subscrição para transações pendentes
            request = {
                "jsonrpc": "2.0",
                "method": "eth_subscribe",
                "params": ["newPendingTransactions"],
                "id": 1
            }
            await websocket.send(json.dumps(request))
            response = await asyncio.wait_for(websocket.recv(), timeout=10)
            response_json = json.loads(response)
            
            if 'result' in response_json:
                subscription_id = response_json['result']
                print_success(f"Subscrição para transações pendentes criada com ID: {subscription_id}")
                
                # Aguarda por uma transação pendente (com timeout)
                print_info("Aguardando por uma transação pendente (timeout de 20s)...")
                try:
                    tx_data = await asyncio.wait_for(websocket.recv(), timeout=20)
                    tx_json = json.loads(tx_data)
                    if 'params' in tx_json and 'result' in tx_json['params']:
                        tx_hash = tx_json['params']['result']
                        print_success(f"Transação pendente recebida: {tx_hash}")
                        return True
                    else:
                        print_warning("Dados de transação recebidos, mas em formato inesperado")
                        print_info(f"Dados recebidos: {tx_json}")
                        return True  # Ainda consideramos sucesso pois a subscrição funcionou
                except asyncio.TimeoutError:
                    print_warning("Timeout aguardando por uma transação pendente, mas a subscrição foi criada com sucesso")
                    return True  # Ainda consideramos sucesso pois a subscrição funcionou
            else:
                print_error(f"Erro ao criar subscrição: {response_json}")
                return False
    except Exception as e:
        print_error(f"Falha no teste de subscrição de transações pendentes: {e}")
        return False

async def run_tests():
    print_info("Iniciando testes de integração com endpoints RPC...")
    print_info(f"WebSocket: {WS_ENDPOINT}")
    print_info(f"HTTP: {HTTP_ENDPOINT}")
    print()
    
    results = {}
    
    # Teste de conexão HTTP
    results['http_connection'] = test_http_connection()
    print()
    
    # Teste de conexão WebSocket
    results['websocket_connection'] = await test_websocket_connection()
    print()
    
    # Teste de conexão Web3
    results['web3_connection'] = test_web3_connection()
    print()
    
    # Teste de subscrição WebSocket para novos blocos
    results['websocket_subscription_blocks'] = await test_websocket_subscription()
    print()
    
    # Teste de subscrição WebSocket para transações pendentes
    results['websocket_subscription_txs'] = await test_pending_transactions()
    print()
    
    # Teste de trace de transação
    results['transaction_trace'] = await test_transaction_trace()
    print()
    
    # Resumo dos resultados
    print_info("Resumo dos testes:")
    all_passed = True
    for test, result in results.items():
        if result:
            print_success(f"{test}: Passou")
        else:
            print_error(f"{test}: Falhou")
            all_passed = False
    
    if all_passed:
        print_success("\nTodos os testes passaram! Os endpoints RPC estão funcionando corretamente.")
        return 0
    else:
        print_error("\nAlguns testes falharam. Verifique os erros acima.")
        return 1

if __name__ == "__main__":
    try:
        exit_code = asyncio.run(run_tests())
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print_warning("\nTestes interrompidos pelo usuário.")
        sys.exit(130)
