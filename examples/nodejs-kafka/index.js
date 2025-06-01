/**
 * Ethernity Kafka Examples - Arquivo Principal
 * 
 * Este arquivo serve como ponto de entrada principal para os exemplos
 * e permite executar diferentes componentes baseado em argumentos.
 */

const { EthernityEventConsumer } = require('./consumer');
const { SubscriptionManager } = require('./subscription-manager');
const { AdvancedEventProcessor } = require('./event-processor');
const { EthernityKafkaTest } = require('./test-consumer');
const winston = require('winston');
require('dotenv').config();

// Configuração do logger
const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.colorize(),
    winston.format.simple()
  ),
  transports: [
    new winston.transports.Console()
  ]
});

/**
 * Exibe ajuda sobre como usar o programa
 */
function showHelp() {
  console.log(`
🚀 Ethernity Kafka Examples

Uso: node index.js <comando> [opções]

Comandos disponíveis:
  consumer              Inicia o consumidor básico de eventos
  subscription-manager  Inicia o gerenciador de inscrições
  event-processor      Inicia o processador avançado de eventos
  test                 Executa testes de conectividade
  help                 Exibe esta ajuda

Exemplos:
  node index.js consumer
  node index.js subscription-manager
  node index.js event-processor
  node index.js test

Variáveis de ambiente importantes:
  KAFKA_BROKERS        Endereços dos brokers Kafka (padrão: localhost:9092)
  KAFKA_GROUP_ID       ID do grupo de consumidores
  LOG_LEVEL           Nível de log (debug, info, warn, error)
  TOPIC_EVENTS        Tópico de eventos (padrão: ethernity-events)

Para mais informações, consulte o README.md
  `);
}

/**
 * Inicia o consumidor básico
 */
async function startConsumer() {
  logger.info('🔍 Iniciando consumidor básico de eventos...');
  
  const consumer = new EthernityEventConsumer();
  
  // Registra handlers personalizados de exemplo
  consumer.registerEventHandler('large_transfer', async (event, topic) => {
    if (event.data.usd_value > 100000) {
      logger.warn('🐋 Transferência de whale detectada:', {
        valor: `$${event.data.usd_value.toLocaleString()}`,
        token: event.data.token.symbol,
        de: event.data.from,
        para: event.data.to
      });
    }
  });
  
  consumer.registerEventHandler('rug_pull_warning', async (event, topic) => {
    logger.error('🚨 ALERTA DE RUG PULL:', {
      token: event.data.token.symbol,
      endereco: event.data.token.address,
      risco: `${(event.data.risk_score * 100).toFixed(1)}%`,
      indicadores: event.data.risk_indicators
    });
  });
  
  // Manipula sinais de sistema
  process.on('SIGINT', async () => {
    logger.info('Recebido SIGINT, parando consumidor...');
    await consumer.stop();
    process.exit(0);
  });
  
  process.on('SIGTERM', async () => {
    logger.info('Recebido SIGTERM, parando consumidor...');
    await consumer.stop();
    process.exit(0);
  });
  
  // Exibe estatísticas periodicamente
  setInterval(() => {
    const stats = consumer.getStats();
    logger.info('📊 Estatísticas do consumidor:', stats);
  }, 30000);
  
  await consumer.start();
}

/**
 * Inicia o gerenciador de inscrições
 */
async function startSubscriptionManager() {
  logger.info('📋 Iniciando gerenciador de inscrições...');
  
  const manager = new SubscriptionManager();
  
  // Manipula sinais de sistema
  process.on('SIGINT', async () => {
    logger.info('Recebido SIGINT, parando gerenciador...');
    await manager.stop();
    process.exit(0);
  });
  
  process.on('SIGTERM', async () => {
    logger.info('Recebido SIGTERM, parando gerenciador...');
    await manager.stop();
    process.exit(0);
  });
  
  // Exibe estatísticas periodicamente
  setInterval(() => {
    const stats = manager.getStats();
    logger.info('📊 Estatísticas do gerenciador:', stats);
  }, 60000);
  
  await manager.start();
}

/**
 * Inicia o processador avançado
 */
async function startEventProcessor() {
  logger.info('⚙️ Iniciando processador avançado de eventos...');
  
  const processor = new AdvancedEventProcessor();
  
  // Manipula sinais de sistema
  process.on('SIGINT', async () => {
    logger.info('Recebido SIGINT, parando processador...');
    await processor.stop();
    process.exit(0);
  });
  
  process.on('SIGTERM', async () => {
    logger.info('Recebido SIGTERM, parando processador...');
    await processor.stop();
    process.exit(0);
  });
  
  // Exibe estatísticas detalhadas periodicamente
  setInterval(() => {
    const stats = processor.getDetailedStats();
    logger.info('📊 Estatísticas detalhadas:', {
      processamento: stats.processing,
      memoria: stats.memory,
      padroesSuspeitos: stats.patterns.length
    });
  }, 60000);
  
  await processor.start();
}

/**
 * Executa testes
 */
async function runTests() {
  logger.info('🧪 Executando testes...');
  
  // Verifica status do Kafka primeiro
  const kafkaRunning = await EthernityKafkaTest.checkKafkaStatus();
  if (!kafkaRunning) {
    logger.error('❌ Kafka não está acessível. Verifique a configuração.');
    process.exit(1);
  }
  
  // Executa suite de testes
  const tester = new EthernityKafkaTest();
  
  try {
    await tester.runAllTests();
    logger.info('✅ Todos os testes passaram!');
    process.exit(0);
  } catch (error) {
    logger.error('❌ Alguns testes falharam:', error.message);
    process.exit(1);
  }
}

/**
 * Função principal
 */
async function main() {
  const command = process.argv[2];
  
  // Exibe informações de inicialização
  logger.info('🚀 Ethernity Kafka Examples');
  logger.info('Configuração:', {
    brokers: process.env.KAFKA_BROKERS || 'localhost:9092',
    groupId: process.env.KAFKA_GROUP_ID || 'ethernity-events-group',
    logLevel: process.env.LOG_LEVEL || 'info'
  });
  
  switch (command) {
    case 'consumer':
      await startConsumer();
      break;
    
    case 'subscription-manager':
      await startSubscriptionManager();
      break;
    
    case 'event-processor':
      await startEventProcessor();
      break;
    
    case 'test':
      await runTests();
      break;
    
    case 'help':
    case '--help':
    case '-h':
      showHelp();
      break;
    
    default:
      if (!command) {
        logger.error('❌ Comando não especificado.');
      } else {
        logger.error(`❌ Comando desconhecido: ${command}`);
      }
      showHelp();
      process.exit(1);
  }
}

// Executa função principal
if (require.main === module) {
  main().catch(error => {
    logger.error('❌ Erro fatal:', error);
    process.exit(1);
  });
}

module.exports = {
  startConsumer,
  startSubscriptionManager,
  startEventProcessor,
  runTests,
  showHelp
};

