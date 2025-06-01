#!/usr/bin/env python3

import asyncio
import json
import sys
import time
from web3 import Web3, AsyncWeb3
from websockets import connect
import logging

# Configuração de logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Endpoints RPC
HTTP_ENDPOINT = "http://116.202.218.100:8545"
WS_ENDPOINT = "ws://116.202.218.100:8546"

async def test_http_connection():
    """Testa a conexão HTTP RPC."""
    try:
        w3 = Web3(Web3.HTTPProvider(HTTP_ENDPOINT))
        block_number = w3.eth.block_number
        logger.info(f"Conexão HTTP bem-sucedida. Número do bloco atual: {block_number}")
        return True
    except Exception as e:
        logger.error(f"Erro na conexão HTTP: {e}")
        return False

async def test_ws_connection():
    """Testa a conexão WebSocket RPC."""
    try:
        async with connect(WS_ENDPOINT) as ws:
            await ws.send(json.dumps({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }))
            response = await ws.recv()
            result = json.loads(response)
            block_number = int(result["result"], 16)
            logger.info(f"Conexão WebSocket bem-sucedida. Número do bloco atual: {block_number}")
            return True
    except Exception as e:
        logger.error(f"Erro na conexão WebSocket: {e}")
        return False

async def test_subscription():
    """Testa a inscrição em novos blocos via WebSocket."""
    try:
        async with connect(WS_ENDPOINT) as ws:
            await ws.send(json.dumps({
                "jsonrpc": "2.0",
                "method": "eth_subscribe",
                "params": ["newHeads"],
                "id": 1
            }))
            
            response = await ws.recv()
            subscription_id = json.loads(response)["result"]
            logger.info(f"Inscrição bem-sucedida. ID: {subscription_id}")
            
            # Aguarda por até 3 blocos ou 60 segundos
            start_time = time.time()
            blocks_received = 0
            
            while blocks_received < 3 and (time.time() - start_time) < 60:
                try:
                    response = await asyncio.wait_for(ws.recv(), timeout=20)
                    blocks_received += 1
                    block_data = json.loads(response)
                    block_number = int(block_data["params"]["result"]["number"], 16)
                    logger.info(f"Bloco recebido: {block_number}")
                except asyncio.TimeoutError:
                    logger.warning("Timeout aguardando por novos blocos")
                    break
            
            if blocks_received > 0:
                logger.info(f"Recebidos {blocks_received} blocos via subscription")
                return True
            else:
                logger.warning("Nenhum bloco recebido via subscription")
                return False
    except Exception as e:
        logger.error(f"Erro no teste de subscription: {e}")
        return False

async def test_transaction_trace():
    """Testa a obtenção de traces de transação."""
    try:
        w3 = Web3(Web3.HTTPProvider(HTTP_ENDPOINT))
        
        # Obtém o bloco mais recente
        latest_block = w3.eth.get_block('latest')
        
        # Verifica se há transações no bloco
        if len(latest_block['transactions']) > 0:
            tx_hash = latest_block['transactions'][0].hex()
            
            # Tenta obter o trace da transação
            trace_request = {
                "jsonrpc": "2.0",
                "method": "debug_traceTransaction",
                "params": [tx_hash, {"tracer": "callTracer"}],
                "id": 1
            }
            
            response = w3.provider.make_request(trace_request["method"], trace_request["params"])
            
            if "result" in response:
                logger.info(f"Trace de transação obtido com sucesso para {tx_hash}")
                return True
            else:
                logger.warning(f"Falha ao obter trace: {response.get('error', 'Erro desconhecido')}")
                return False
        else:
            logger.warning("Nenhuma transação encontrada no bloco mais recente")
            return False
    except Exception as e:
        logger.error(f"Erro no teste de trace de transação: {e}")
        return False

async def test_pending_transactions():
    """Testa a inscrição em transações pendentes via WebSocket."""
    try:
        async with connect(WS_ENDPOINT) as ws:
            await ws.send(json.dumps({
                "jsonrpc": "2.0",
                "method": "eth_subscribe",
                "params": ["newPendingTransactions"],
                "id": 1
            }))
            
            response = await ws.recv()
            subscription_id = json.loads(response)["result"]
            logger.info(f"Inscrição em transações pendentes bem-sucedida. ID: {subscription_id}")
            
            # Aguarda por até 5 transações ou 30 segundos
            start_time = time.time()
            txs_received = 0
            
            while txs_received < 5 and (time.time() - start_time) < 30:
                try:
                    response = await asyncio.wait_for(ws.recv(), timeout=10)
                    txs_received += 1
                    tx_data = json.loads(response)
                    tx_hash = tx_data["params"]["result"]
                    logger.info(f"Transação pendente recebida: {tx_hash}")
                except asyncio.TimeoutError:
                    logger.warning("Timeout aguardando por transações pendentes")
                    break
            
            if txs_received > 0:
                logger.info(f"Recebidas {txs_received} transações pendentes via subscription")
                return True
            else:
                logger.warning("Nenhuma transação pendente recebida via subscription")
                return False
    except Exception as e:
        logger.error(f"Erro no teste de subscription de transações pendentes: {e}")
        return False

async def run_tests():
    """Executa todos os testes de integração."""
    results = {}
    
    logger.info("Iniciando testes de integração com endpoints RPC...")
    
    # Teste de conexão HTTP
    results["http_connection"] = await test_http_connection()
    
    # Teste de conexão WebSocket
    results["ws_connection"] = await test_ws_connection()
    
    # Teste de subscription de blocos
    results["block_subscription"] = await test_subscription()
    
    # Teste de trace de transação
    results["transaction_trace"] = await test_transaction_trace()
    
    # Teste de subscription de transações pendentes
    results["pending_transactions"] = await test_pending_transactions()
    
    # Exibe resultados
    logger.info("\n=== RESULTADOS DOS TESTES ===")
    for test, result in results.items():
        status = "PASSOU" if result else "FALHOU"
        logger.info(f"{test}: {status}")
    
    # Verifica se todos os testes passaram
    all_passed = all(results.values())
    logger.info(f"\nStatus geral: {'TODOS OS TESTES PASSARAM' if all_passed else 'ALGUNS TESTES FALHARAM'}")
    
    return all_passed

if __name__ == "__main__":
    try:
        success = asyncio.run(run_tests())
        sys.exit(0 if success else 1)
    except KeyboardInterrupt:
        logger.info("Testes interrompidos pelo usuário")
        sys.exit(130)
