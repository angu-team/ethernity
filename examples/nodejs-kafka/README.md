# Ethernity Kafka Examples - Node.js

Este diretório contém exemplos completos de como usar Node.js para se inscrever em filas Kafka e processar eventos do Ethernity Workspace em tempo real.

## 📋 Pré-requisitos

- Node.js 16+ 
- Kafka rodando (local ou remoto)
- NPM ou Yarn

## 🚀 Instalação

1. Instale as dependências:
```bash
npm install
```

2. Configure as variáveis de ambiente:
```bash
cp .env.example .env
# Edite o arquivo .env com suas configurações
```

3. Teste a conectividade:
```bash
npm test
```

## 📁 Estrutura dos Arquivos

### `consumer.js` - Consumidor Básico de Eventos
Consumidor simples que se inscreve em eventos Kafka e processa diferentes tipos de eventos blockchain.

**Características:**
- Processamento de eventos em tempo real
- Handlers específicos para cada tipo de evento
- Sistema de logging estruturado
- Estatísticas de processamento
- Parada graciosa

**Uso:**
```bash
npm run consumer
```

### `subscription-manager.js` - Gerenciador de Inscrições
Sistema avançado para gerenciar inscrições de eventos dinamicamente através de comandos Kafka.

**Características:**
- Criação, atualização e remoção de inscrições
- Validação de dados com Joi
- Filtros personalizáveis
- Múltiplos métodos de notificação
- Rate limiting

**Uso:**
```bash
npm run subscription-manager
```

### `event-processor.js` - Processador Avançado
Processador sofisticado que analisa padrões, detecta anomalias e processa eventos em lotes.

**Características:**
- Processamento em lotes para alta performance
- Detecção de padrões suspeitos
- Análise de correlações entre eventos
- Sistema de alertas
- Detecção de MEV e ataques

**Uso:**
```bash
npm run event-processor
```

### `test-consumer.js` - Testes e Validação
Suite de testes para validar conectividade e funcionalidade dos exemplos.

**Uso:**
```bash
npm test
```

## 🔧 Configuração

### Variáveis de Ambiente (.env)

```env
# Configurações do Kafka
KAFKA_BROKERS=localhost:9092
KAFKA_CLIENT_ID=ethernity-nodejs-consumer
KAFKA_GROUP_ID=ethernity-events-group

# Tópicos Kafka
TOPIC_EVENTS=ethernity-events
TOPIC_SUBSCRIPTIONS=ethernity-subscriptions
TOPIC_NOTIFICATIONS=ethernity-notifications

# Configurações de logging
LOG_LEVEL=info

# Configurações de retry
MAX_RETRIES=3
RETRY_DELAY_MS=1000

# Configurações de processamento
BATCH_SIZE=100
PROCESSING_TIMEOUT_MS=30000
```

## 📊 Tipos de Eventos Suportados

### 1. `erc20_created` - Criação de Token ERC20
```json
{
  "event_type": "erc20_created",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "contract_address": "0x...",
    "creator": "0x...",
    "name": "Token Name",
    "symbol": "TKN",
    "total_supply": "1000000000000000000000000"
  }
}
```

### 2. `token_swap` - Swap de Tokens
```json
{
  "event_type": "token_swap",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "user": "0x...",
    "token_in": {
      "address": "0x...",
      "symbol": "ETH",
      "decimals": 18
    },
    "token_out": {
      "address": "0x...",
      "symbol": "USDC",
      "decimals": 6
    },
    "amount_in": "1000000000000000000",
    "amount_out": "2500000000",
    "dex_protocol": "UniswapV3"
  }
}
```

### 3. `large_transfer` - Transferência Grande
```json
{
  "event_type": "large_transfer",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "from": "0x...",
    "to": "0x...",
    "amount": "50000000000000000000000",
    "usd_value": 50000,
    "token": {
      "address": "0x...",
      "symbol": "USDT",
      "decimals": 6
    }
  }
}
```

### 4. `liquidation` - Liquidação
```json
{
  "event_type": "liquidation",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "liquidated_user": "0x...",
    "liquidator": "0x...",
    "collateral_token": {
      "address": "0x...",
      "symbol": "ETH"
    },
    "debt_token": {
      "address": "0x...",
      "symbol": "USDC"
    },
    "liquidated_amount": "1000000000000000000"
  }
}
```

### 5. `rug_pull_warning` - Alerta de Rug Pull
```json
{
  "event_type": "rug_pull_warning",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "token": {
      "address": "0x...",
      "symbol": "SCAM"
    },
    "deployer": "0x...",
    "risk_score": 0.95,
    "risk_indicators": [
      "honeypot_detected",
      "liquidity_removed",
      "ownership_not_renounced"
    ]
  }
}
```

### 6. `mev_activity` - Atividade MEV
```json
{
  "event_type": "mev_activity",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "mev_type": "sandwich_attack",
    "bot_address": "0x...",
    "profit_usd": 1500,
    "gas_used": 250000,
    "victim_tx": "0x..."
  }
}
```

### 7. `flash_loan` - Flash Loan
```json
{
  "event_type": "flash_loan",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "user": "0x...",
    "token": {
      "address": "0x...",
      "symbol": "DAI"
    },
    "amount": "1000000000000000000000000",
    "fee": "900000000000000000000",
    "protocol": "Aave"
  }
}
```

### 8. `governance_event` - Evento de Governança
```json
{
  "event_type": "governance_event",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "governance_type": "proposal_created",
    "proposal_id": "123",
    "proposer": "0x...",
    "description": "Increase protocol fee to 0.3%",
    "voting_power": "1000000000000000000000"
  }
}
```

## 🎯 Exemplos de Uso

### Consumidor Básico com Handler Personalizado

```javascript
const { EthernityEventConsumer } = require('./consumer');

const consumer = new EthernityEventConsumer();

// Registra handler personalizado para swaps grandes
consumer.registerEventHandler('token_swap', async (event, topic) => {
  const { amount_in, token_in, usd_value } = event.data;
  
  if (usd_value > 100000) { // Swaps > $100k
    console.log('🚨 Swap grande detectado:', {
      valor: usd_value,
      token: token_in.symbol,
      usuario: event.data.user
    });
    
    // Enviar notificação, salvar no banco, etc.
    await sendAlert('large_swap', event.data);
  }
});

await consumer.start();
```

### Criando Inscrições Dinamicamente

```javascript
const { createSubscriptionCommand } = require('./subscription-manager');

// Cria inscrição para transferências grandes
const subscription = createSubscriptionCommand('create', {
  data: {
    user_id: 'whale-tracker-001',
    event_type: 'large_transfer',
    filters: {
      general: {
        min_value_usd: 1000000, // Apenas transferências > $1M
        include_mempool: true,
        address_whitelist: [
          '0x...' // Endereços específicos para monitorar
        ]
      }
    },
    notification_config: {
      method: 'webhook',
      webhook_url: 'https://api.myapp.com/webhooks/whale-alert',
      retry_policy: {
        max_retries: 5,
        initial_delay: 1000
      }
    },
    rate_limit: {
      events_per_minute: 10,
      events_per_hour: 100
    }
  }
});

// Envia comando via Kafka
await producer.send({
  topic: 'ethernity-subscriptions',
  messages: [{
    key: 'create',
    value: JSON.stringify(subscription)
  }]
});
```

### Processamento Avançado com Detecção de Padrões

```javascript
const { AdvancedEventProcessor } = require('./event-processor');

const processor = new AdvancedEventProcessor();

// O processador automaticamente detecta:
// - Sandwich attacks
// - Atividade MEV suspeita
// - Padrões de rug pull
// - Volumes anômalos
// - Correlações entre eventos

await processor.start();

// Obtém estatísticas em tempo real
setInterval(() => {
  const stats = processor.getDetailedStats();
  console.log('📊 Estatísticas:', stats);
}, 60000);
```

## 🔍 Monitoramento e Logs

Todos os exemplos incluem logging estruturado com Winston:

- **Console**: Logs coloridos para desenvolvimento
- **Arquivo**: Logs persistentes em arquivos
- **Níveis**: debug, info, warn, error
- **Formato**: JSON estruturado com timestamps

### Configuração de Log Level

```bash
# .env
LOG_LEVEL=debug  # Para desenvolvimento
LOG_LEVEL=info   # Para produção
LOG_LEVEL=warn   # Apenas avisos e erros
```

## 📈 Performance e Escalabilidade

### Configurações Recomendadas

**Para Alto Volume (>1000 eventos/segundo):**
```env
BATCH_SIZE=500
PROCESSING_TIMEOUT_MS=60000
KAFKA_GROUP_ID=ethernity-high-volume-group
```

**Para Baixa Latência:**
```env
BATCH_SIZE=10
PROCESSING_TIMEOUT_MS=5000
```

### Múltiplas Instâncias

Para escalar horizontalmente, execute múltiplas instâncias com o mesmo `KAFKA_GROUP_ID`. O Kafka distribuirá automaticamente as partições entre as instâncias.

## 🚨 Tratamento de Erros

Todos os exemplos incluem:

- **Retry automático** com backoff exponencial
- **Dead letter queues** para mensagens problemáticas
- **Circuit breakers** para serviços externos
- **Graceful shutdown** em sinais do sistema
- **Health checks** para monitoramento

## 🔒 Segurança

### Configuração SASL/SSL (Produção)

```javascript
const kafka = new Kafka({
  clientId: 'ethernity-secure-client',
  brokers: ['kafka1:9093', 'kafka2:9093'],
  ssl: true,
  sasl: {
    mechanism: 'SCRAM-SHA-256',
    username: process.env.KAFKA_USERNAME,
    password: process.env.KAFKA_PASSWORD
  }
});
```

### Validação de Dados

Todos os dados são validados com Joi antes do processamento:

```javascript
const eventSchema = Joi.object({
  event_type: Joi.string().required(),
  timestamp: Joi.date().iso().required(),
  data: Joi.object().required()
});
```

## 🐛 Troubleshooting

### Problemas Comuns

1. **Kafka não conecta:**
   - Verifique se o Kafka está rodando
   - Confirme o endereço dos brokers
   - Teste conectividade de rede

2. **Mensagens não são consumidas:**
   - Verifique se os tópicos existem
   - Confirme o group ID
   - Verifique offsets

3. **Performance baixa:**
   - Aumente o batch size
   - Use múltiplas partições
   - Otimize processamento

### Comandos Úteis

```bash
# Testa conectividade
npm test

# Executa com logs debug
LOG_LEVEL=debug npm run consumer

# Monitora estatísticas
npm run consumer 2>&1 | grep "Estatísticas"

# Verifica tópicos Kafka
kafka-topics.sh --list --bootstrap-server localhost:9092
```

## 📚 Recursos Adicionais

- [Documentação KafkaJS](https://kafka.js.org/)
- [Kafka Documentation](https://kafka.apache.org/documentation/)
- [Winston Logging](https://github.com/winstonjs/winston)
- [Joi Validation](https://joi.dev/)

## 🤝 Contribuindo

1. Fork o projeto
2. Crie uma branch para sua feature
3. Commit suas mudanças
4. Push para a branch
5. Abra um Pull Request

## 📄 Licença

MIT License - veja o arquivo LICENSE para detalhes.

