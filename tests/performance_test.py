#!/usr/bin/env python3

import asyncio
import json
import websockets
import requests
from web3 import Web3
import time
import sys
import matplotlib.pyplot as plt
import numpy as np
import psutil
import os
import threading
import concurrent.futures

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

class PerformanceMonitor:
    def __init__(self):
        self.cpu_usage = []
        self.memory_usage = []
        self.timestamps = []
        self.running = False
        self.thread = None
    
    def start(self):
        self.running = True
        self.thread = threading.Thread(target=self._monitor)
        self.thread.daemon = True
        self.thread.start()
    
    def stop(self):
        self.running = False
        if self.thread:
            self.thread.join(timeout=1)
    
    def _monitor(self):
        start_time = time.time()
        while self.running:
            self.cpu_usage.append(psutil.cpu_percent())
            self.memory_usage.append(psutil.Process(os.getpid()).memory_info().rss / 1024 / 1024)  # MB
            self.timestamps.append(time.time() - start_time)
            time.sleep(0.5)
    
    def plot(self, filename_prefix):
        plt.figure(figsize=(12, 10))
        
        # CPU Usage
        plt.subplot(2, 1, 1)
        plt.plot(self.timestamps, self.cpu_usage)
        plt.title('CPU Usage')
        plt.xlabel('Time (seconds)')
        plt.ylabel('CPU (%)')
        plt.grid(True)
        
        # Memory Usage
        plt.subplot(2, 1, 2)
        plt.plot(self.timestamps, self.memory_usage)
        plt.title('Memory Usage')
        plt.xlabel('Time (seconds)')
        plt.ylabel('Memory (MB)')
        plt.grid(True)
        
        plt.tight_layout()
        plt.savefig(f"{filename_prefix}_resource_usage.png")
        print_info(f"Resource usage plot saved to {filename_prefix}_resource_usage.png")

async def test_websocket_latency():
    print_info("Testando latência da conexão WebSocket...")
    latencies = []
    
    try:
        async with websockets.connect(WS_ENDPOINT) as websocket:
            for i in range(10):
                request = {
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": [],
                    "id": i + 1
                }
                
                start_time = time.time()
                await websocket.send(json.dumps(request))
                response = await asyncio.wait_for(websocket.recv(), timeout=10)
                end_time = time.time()
                
                latency = (end_time - start_time) * 1000  # ms
                latencies.append(latency)
                print_info(f"Requisição {i+1}: {latency:.2f} ms")
                
                # Pequena pausa para não sobrecarregar
                await asyncio.sleep(0.5)
            
            avg_latency = sum(latencies) / len(latencies)
            print_success(f"Latência média: {avg_latency:.2f} ms")
            return latencies
    except Exception as e:
        print_error(f"Falha no teste de latência: {e}")
        return []

async def test_batch_processing():
    print_info("Testando processamento em lote...")
    
    # Tamanhos de lote para testar
    batch_sizes = [1, 10, 50, 100]
    batch_results = {}
    
    try:
        w3 = Web3(Web3.HTTPProvider(HTTP_ENDPOINT))
        latest_block = w3.eth.block_number
        
        for batch_size in batch_sizes:
            print_info(f"Testando lote de {batch_size} requisições...")
            
            # Preparar lote de requisições
            batch = []
            for i in range(batch_size):
                block_num = latest_block - i if latest_block - i > 0 else 1
                batch.append({
                    "jsonrpc": "2.0",
                    "method": "eth_getBlockByNumber",
                    "params": [hex(block_num), False],
                    "id": i + 1
                })
            
            # Enviar lote
            start_time = time.time()
            response = requests.post(
                HTTP_ENDPOINT,
                json=batch,
                timeout=30
            )
            end_time = time.time()
            
            if response.status_code == 200:
                results = response.json()
                processing_time = (end_time - start_time) * 1000  # ms
                avg_time_per_request = processing_time / batch_size
                
                batch_results[batch_size] = {
                    'total_time': processing_time,
                    'avg_time_per_request': avg_time_per_request
                }
                
                print_success(f"Lote de {batch_size} processado em {processing_time:.2f} ms")
                print_info(f"Tempo médio por requisição: {avg_time_per_request:.2f} ms")
            else:
                print_error(f"Falha no processamento do lote de {batch_size}: {response.status_code}")
        
        return batch_results
    except Exception as e:
        print_error(f"Falha no teste de processamento em lote: {e}")
        return {}

async def test_parallel_requests():
    print_info("Testando requisições paralelas...")
    
    # Número de requisições paralelas
    num_requests = [10, 50, 100, 200]
    results = {}
    
    for n in num_requests:
        print_info(f"Testando {n} requisições paralelas...")
        
        async def make_request(i):
            try:
                response = requests.post(
                    HTTP_ENDPOINT,
                    json={
                        "jsonrpc": "2.0",
                        "method": "eth_blockNumber",
                        "params": [],
                        "id": i
                    },
                    timeout=10
                )
                return response.status_code == 200
            except Exception:
                return False
        
        # Criar tarefas
        start_time = time.time()
        with concurrent.futures.ThreadPoolExecutor(max_workers=n) as executor:
            futures = [executor.submit(make_request, i) for i in range(n)]
            completed = [future.result() for future in concurrent.futures.as_completed(futures)]
        
        end_time = time.time()
        
        # Calcular resultados
        success_count = sum(1 for result in completed if result)
        total_time = (end_time - start_time) * 1000  # ms
        
        results[n] = {
            'total_time': total_time,
            'success_rate': success_count / n * 100,
            'requests_per_second': n / (total_time / 1000)
        }
        
        print_success(f"{n} requisições paralelas: {success_count} sucessos ({results[n]['success_rate']:.1f}%)")
        print_info(f"Tempo total: {total_time:.2f} ms")
        print_info(f"Taxa: {results[n]['requests_per_second']:.2f} req/s")
    
    return results

async def test_memory_usage_under_load():
    print_info("Testando uso de memória sob carga...")
    
    # Iniciar monitor de performance
    monitor = PerformanceMonitor()
    monitor.start()
    
    try:
        # Teste de carga com WebSocket
        print_info("Iniciando teste de carga com WebSocket...")
        
        # Conectar a múltiplos WebSockets simultaneamente
        connections = []
        for i in range(5):
            try:
                ws = await websockets.connect(WS_ENDPOINT)
                connections.append(ws)
                print_info(f"Conexão WebSocket {i+1} estabelecida")
            except Exception as e:
                print_error(f"Falha ao estabelecer conexão WebSocket {i+1}: {e}")
        
        # Subscrever a eventos em cada conexão
        subscriptions = []
        for i, ws in enumerate(connections):
            try:
                # Alternar entre subscrições de blocos e transações
                method = "newHeads" if i % 2 == 0 else "newPendingTransactions"
                
                request = {
                    "jsonrpc": "2.0",
                    "method": "eth_subscribe",
                    "params": [method],
                    "id": i + 1
                }
                
                await ws.send(json.dumps(request))
                response = await asyncio.wait_for(ws.recv(), timeout=10)
                response_json = json.loads(response)
                
                if 'result' in response_json:
                    subscriptions.append((ws, response_json['result'], method))
                    print_success(f"Subscrição {i+1} ({method}) criada com ID: {response_json['result']}")
                else:
                    print_error(f"Falha ao criar subscrição {i+1}: {response_json}")
            except Exception as e:
                print_error(f"Erro na subscrição {i+1}: {e}")
        
        # Executar requisições HTTP em paralelo enquanto recebe eventos WebSocket
        print_info("Executando requisições HTTP em paralelo...")
        
        async def process_websocket_events():
            event_counts = {ws: 0 for ws, _, _ in subscriptions}
            start_time = time.time()
            duration = 30  # segundos
            
            while time.time() - start_time < duration:
                for ws, sub_id, method in subscriptions:
                    try:
                        # Verificar se há eventos disponíveis com um timeout curto
                        try:
                            event = await asyncio.wait_for(ws.recv(), timeout=0.1)
                            event_counts[ws] += 1
                            
                            # Exibir apenas alguns eventos para não poluir a saída
                            if event_counts[ws] <= 3:
                                event_json = json.loads(event)
                                if 'params' in event_json and 'result' in event_json['params']:
                                    if method == "newHeads":
                                        block_number = int(event_json['params']['result']['number'], 16)
                                        print_info(f"Bloco recebido: {block_number}")
                                    else:
                                        tx_hash = event_json['params']['result']
                                        print_info(f"Transação pendente: {tx_hash[:20]}...")
                        except asyncio.TimeoutError:
                            # Sem eventos disponíveis, continuar
                            pass
                    except Exception as e:
                        print_error(f"Erro ao processar eventos: {e}")
                
                # Pequena pausa para não sobrecarregar a CPU
                await asyncio.sleep(0.1)
            
            # Exibir contagem total de eventos
            for i, (ws, _, method) in enumerate(subscriptions):
                print_success(f"Conexão {i+1} ({method}): {event_counts[ws]} eventos recebidos")
        
        async def make_http_requests():
            start_time = time.time()
            duration = 30  # segundos
            request_count = 0
            
            while time.time() - start_time < duration:
                try:
                    response = requests.post(
                        HTTP_ENDPOINT,
                        json={"jsonrpc": "2.0", "method": "eth_blockNumber", "params": [], "id": request_count + 1},
                        timeout=5
                    )
                    
                    if response.status_code == 200:
                        request_count += 1
                        
                        # Exibir apenas algumas respostas para não poluir a saída
                        if request_count <= 5 or request_count % 50 == 0:
                            response_json = response.json()
                            if 'result' in response_json:
                                block_number = int(response_json['result'], 16)
                                print_info(f"Requisição HTTP {request_count}: Bloco atual {block_number}")
                    else:
                        print_error(f"Falha na requisição HTTP: {response.status_code}")
                except Exception as e:
                    print_error(f"Erro na requisição HTTP: {e}")
                
                # Pequena pausa para não sobrecarregar
                await asyncio.sleep(0.1)
            
            print_success(f"Total de requisições HTTP: {request_count} em {duration} segundos")
            print_info(f"Taxa média: {request_count / duration:.2f} req/s")
        
        # Executar ambas as tarefas em paralelo
        print_info(f"Executando teste de carga por 30 segundos...")
        await asyncio.gather(
            process_websocket_events(),
            make_http_requests()
        )
        
        # Fechar conexões WebSocket
        for ws in connections:
            await ws.close()
        
        print_success("Teste de carga concluído!")
        
    finally:
        # Parar monitor e gerar gráficos
        monitor.stop()
        monitor.plot("memory_usage_load_test")
    
    return True

async def run_performance_tests():
    print_info("Iniciando testes de performance...")
    print_info(f"WebSocket: {WS_ENDPOINT}")
    print_info(f"HTTP: {HTTP_ENDPOINT}")
    print()
    
    results = {}
    
    # Teste de latência WebSocket
    print_info("=== TESTE DE LATÊNCIA WEBSOCKET ===")
    results['websocket_latency'] = await test_websocket_latency()
    print()
    
    # Teste de processamento em lote
    print_info("=== TESTE DE PROCESSAMENTO EM LOTE ===")
    results['batch_processing'] = await test_batch_processing()
    print()
    
    # Teste de requisições paralelas
    print_info("=== TESTE DE REQUISIÇÕES PARALELAS ===")
    results['parallel_requests'] = await test_parallel_requests()
    print()
    
    # Teste de uso de memória sob carga
    print_info("=== TESTE DE USO DE MEMÓRIA SOB CARGA ===")
    results['memory_usage'] = await test_memory_usage_under_load()
    print()
    
    # Gerar relatório
    print_info("=== RELATÓRIO DE PERFORMANCE ===")
    
    if results['websocket_latency']:
        avg_latency = sum(results['websocket_latency']) / len(results['websocket_latency'])
        print_success(f"Latência média WebSocket: {avg_latency:.2f} ms")
    
    if results['batch_processing']:
        print_success("Processamento em lote:")
        for batch_size, data in results['batch_processing'].items():
            print_info(f"  Lote de {batch_size}: {data['total_time']:.2f} ms total, {data['avg_time_per_request']:.2f} ms por requisição")
    
    if results['parallel_requests']:
        print_success("Requisições paralelas:")
        for n, data in results['parallel_requests'].items():
            print_info(f"  {n} requisições: {data['requests_per_second']:.2f} req/s, {data['success_rate']:.1f}% sucesso")
    
    print_success("Testes de performance concluídos!")
    return 0

if __name__ == "__main__":
    try:
        exit_code = asyncio.run(run_performance_tests())
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print_warning("\nTestes interrompidos pelo usuário.")
        sys.exit(130)
