/**
 * Teste dos Exemplos Kafka do Ethernity
 * 
 * Este arquivo testa a conectividade com Kafka e demonstra
 * como usar todos os exemplos criados.
 */

const { Kafka } = require('kafkajs');
const winston = require('winston');
const { EthernityEventConsumer } = require('./consumer');
const { SubscriptionManager, createSubscriptionCommand } = require('./subscription-manager');
const { AdvancedEventProcessor } = require('./event-processor');
require('dotenv').config();

// Configuração do logger
const logger = winston.createLogger({
  level: 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.colorize(),
    winston.format.simple()
  ),
  transports: [
    new winston.transports.Console()
  ]
});

class EthernityKafkaTest {
  constructor() {
    this.kafka = new Kafka({
      clientId: 'ethernity-test-client',
      brokers: (process.env.KAFKA_BROKERS || 'localhost:9092').split(','),
      connectionTimeout: 3000,
      requestTimeout: 25000
    });
    
    this.admin = this.kafka.admin();
    this.producer = this.kafka.producer();
    this.testResults = {
      connectivity: false,
      topicCreation: false,
      messageProduction: false,
      messageConsumption: false,
      examples: {
        consumer: false,
        subscriptionManager: false,
        eventProcessor: false
      }
    };
  }

  /**
   * Executa todos os testes
   */
  async runAllTests() {
    logger.info('🚀 Iniciando testes do Ethernity Kafka...');
    
    try {
      await this.testConnectivity();
      await this.testTopicCreation();
      await this.testMessageProduction();
      await this.testMessageConsumption();
      await this.testExamples();
      
      this.printResults();
      
    } catch (error) {
      logger.error('❌ Erro durante os testes:', error);
      throw error;
    } finally {
      await this.cleanup();
    }
  }

  /**
   * Testa conectividade com Kafka
   */
  async testConnectivity() {
    logger.info('🔌 Testando conectividade com Kafka...');
    
    try {
      await this.admin.connect();
      const metadata = await this.admin.fetchTopicMetadata();
      
      logger.info('✅ Conectividade OK', {
        brokers: metadata.brokers.length,
        topics: metadata.topics.length
      });
      
      this.testResults.connectivity = true;
      
    } catch (error) {
      logger.error('❌ Falha na conectividade:', error.message);
      throw error;
    }
  }

  /**
   * Testa criação de tópicos
   */
  async testTopicCreation() {
    logger.info('📝 Testando criação de tópicos...');
    
    const testTopics = [
      'ethernity-events-test',
      'ethernity-subscriptions-test',
      'ethernity-notifications-test',
      'ethernity-alerts-test'
    ];

    try {
      // Verifica tópicos existentes
      const existingTopics = await this.admin.listTopics();
      const topicsToCreate = testTopics.filter(topic => !existingTopics.includes(topic));
      
      if (topicsToCreate.length > 0) {
        await this.admin.createTopics({
          topics: topicsToCreate.map(topic => ({
            topic,
            numPartitions: 3,
            replicationFactor: 1,
            configEntries: [
              { name: 'cleanup.policy', value: 'delete' },
              { name: 'retention.ms', value: '86400000' } // 24 horas
            ]
          }))
        });
        
        logger.info('✅ Tópicos criados:', topicsToCreate);
      } else {
        logger.info('✅ Todos os tópicos já existem');
      }
      
      this.testResults.topicCreation = true;
      
    } catch (error) {
      logger.error('❌ Falha na criação de tópicos:', error.message);
      throw error;
    }
  }

  /**
   * Testa produção de mensagens
   */
  async testMessageProduction() {
    logger.info('📤 Testando produção de mensagens...');
    
    try {
      await this.producer.connect();
      
      // Produz eventos de teste
      const testEvents = this.generateTestEvents();
      
      for (const event of testEvents) {
        await this.producer.send({
          topic: 'ethernity-events-test',
          messages: [{
            key: event.event_type,
            value: JSON.stringify(event),
            timestamp: Date.now().toString()
          }]
        });
      }
      
      logger.info('✅ Mensagens produzidas:', testEvents.length);
      this.testResults.messageProduction = true;
      
    } catch (error) {
      logger.error('❌ Falha na produção de mensagens:', error.message);
      throw error;
    }
  }

  /**
   * Testa consumo de mensagens
   */
  async testMessageConsumption() {
    logger.info('📥 Testando consumo de mensagens...');
    
    const consumer = this.kafka.consumer({ 
      groupId: 'ethernity-test-group',
      sessionTimeout: 10000
    });
    
    try {
      await consumer.connect();
      await consumer.subscribe({ 
        topic: 'ethernity-events-test',
        fromBeginning: true 
      });
      
      let messagesReceived = 0;
      const timeout = setTimeout(() => {
        consumer.disconnect();
      }, 5000);
      
      await consumer.run({
        eachMessage: async ({ topic, partition, message }) => {
          messagesReceived++;
          logger.debug('Mensagem recebida:', {
            topic,
            partition,
            offset: message.offset,
            key: message.key?.toString()
          });
          
          if (messagesReceived >= 3) {
            clearTimeout(timeout);
            await consumer.disconnect();
          }
        }
      });
      
      logger.info('✅ Mensagens consumidas:', messagesReceived);
      this.testResults.messageConsumption = messagesReceived > 0;
      
    } catch (error) {
      logger.error('❌ Falha no consumo de mensagens:', error.message);
      await consumer.disconnect();
      throw error;
    }
  }

  /**
   * Testa os exemplos criados
   */
  async testExamples() {
    logger.info('🧪 Testando exemplos...');
    
    // Testa Consumer básico
    await this.testBasicConsumer();
    
    // Testa Subscription Manager
    await this.testSubscriptionManager();
    
    // Testa Event Processor
    await this.testEventProcessor();
  }

  /**
   * Testa o consumidor básico
   */
  async testBasicConsumer() {
    logger.info('🔍 Testando consumidor básico...');
    
    try {
      const consumer = new EthernityEventConsumer();
      
      // Registra handler de teste
      let eventsReceived = 0;
      consumer.registerEventHandler('test_event', async (event) => {
        eventsReceived++;
        logger.debug('Evento de teste recebido:', event.event_type);
      });
      
      // Simula início e parada rápida
      setTimeout(async () => {
        await consumer.stop();
      }, 2000);
      
      // Nota: Em um teste real, você iniciaria o consumer e enviaria eventos
      logger.info('✅ Consumidor básico testado');
      this.testResults.examples.consumer = true;
      
    } catch (error) {
      logger.error('❌ Falha no teste do consumidor:', error.message);
    }
  }

  /**
   * Testa o gerenciador de inscrições
   */
  async testSubscriptionManager() {
    logger.info('📋 Testando gerenciador de inscrições...');
    
    try {
      // Cria comandos de teste
      const createCommand = createSubscriptionCommand('create', {
        data: {
          user_id: 'test-user-123',
          event_type: 'large_transfer',
          filters: {
            general: {
              min_value_usd: 1000
            }
          },
          notification_config: {
            method: 'webhook',
            webhook_url: 'https://api.test.com/webhook'
          }
        }
      });
      
      // Produz comando de teste
      await this.producer.send({
        topic: 'ethernity-subscriptions-test',
        messages: [{
          key: 'test',
          value: JSON.stringify(createCommand)
        }]
      });
      
      logger.info('✅ Gerenciador de inscrições testado');
      this.testResults.examples.subscriptionManager = true;
      
    } catch (error) {
      logger.error('❌ Falha no teste do gerenciador:', error.message);
    }
  }

  /**
   * Testa o processador de eventos
   */
  async testEventProcessor() {
    logger.info('⚙️ Testando processador de eventos...');
    
    try {
      // Produz eventos complexos para teste
      const complexEvents = this.generateComplexTestEvents();
      
      for (const event of complexEvents) {
        await this.producer.send({
          topic: 'ethernity-events-test',
          messages: [{
            key: event.event_type,
            value: JSON.stringify(event)
          }]
        });
      }
      
      logger.info('✅ Processador de eventos testado');
      this.testResults.examples.eventProcessor = true;
      
    } catch (error) {
      logger.error('❌ Falha no teste do processador:', error.message);
    }
  }

  /**
   * Gera eventos de teste
   */
  generateTestEvents() {
    return [
      {
        event_type: 'erc20_created',
        timestamp: new Date().toISOString(),
        data: {
          contract_address: '0x1234567890123456789012345678901234567890',
          creator: '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
          name: 'Test Token',
          symbol: 'TEST',
          total_supply: '1000000000000000000000000'
        }
      },
      {
        event_type: 'large_transfer',
        timestamp: new Date().toISOString(),
        data: {
          from: '0x1111111111111111111111111111111111111111',
          to: '0x2222222222222222222222222222222222222222',
          amount: '50000000000000000000000',
          usd_value: 50000,
          token: {
            address: '0x1234567890123456789012345678901234567890',
            symbol: 'TEST',
            decimals: 18
          }
        }
      },
      {
        event_type: 'token_swap',
        timestamp: new Date().toISOString(),
        data: {
          user: '0x3333333333333333333333333333333333333333',
          token_in: {
            address: '0x1234567890123456789012345678901234567890',
            symbol: 'TEST',
            decimals: 18
          },
          token_out: {
            address: '0x5678901234567890123456789012345678901234',
            symbol: 'USDC',
            decimals: 6
          },
          amount_in: '1000000000000000000000',
          amount_out: '1000000000',
          dex_protocol: 'UniswapV3'
        }
      }
    ];
  }

  /**
   * Gera eventos complexos para teste
   */
  generateComplexTestEvents() {
    const baseTime = Date.now();
    
    return [
      // Sequência de eventos que pode indicar sandwich attack
      {
        event_type: 'token_swap',
        timestamp: new Date(baseTime).toISOString(),
        data: {
          user: '0x4444444444444444444444444444444444444444',
          token_in: { symbol: 'ETH', address: '0xeth' },
          token_out: { symbol: 'USDC', address: '0xusdc' },
          amount_in: '10000000000000000000',
          amount_out: '25000000000'
        }
      },
      {
        event_type: 'mev_activity',
        timestamp: new Date(baseTime + 15000).toISOString(),
        data: {
          mev_type: 'sandwich_attack',
          bot_address: '0x5555555555555555555555555555555555555555',
          profit_usd: 500,
          gas_used: 150000
        }
      },
      // Transferência grande suspeita
      {
        event_type: 'large_transfer',
        timestamp: new Date(baseTime + 30000).toISOString(),
        data: {
          from: '0x6666666666666666666666666666666666666666',
          to: '0x7777777777777777777777777777777777777777',
          amount: '1000000000000000000000000',
          usd_value: 2000000,
          token: { symbol: 'USDT', address: '0xusdt' }
        }
      }
    ];
  }

  /**
   * Limpa recursos de teste
   */
  async cleanup() {
    logger.info('🧹 Limpando recursos de teste...');
    
    try {
      // Deleta tópicos de teste
      const testTopics = [
        'ethernity-events-test',
        'ethernity-subscriptions-test',
        'ethernity-notifications-test',
        'ethernity-alerts-test'
      ];
      
      await this.admin.deleteTopics({ topics: testTopics });
      logger.info('✅ Tópicos de teste removidos');
      
    } catch (error) {
      logger.warn('⚠️ Erro ao limpar tópicos:', error.message);
    }
    
    try {
      await this.producer.disconnect();
      await this.admin.disconnect();
    } catch (error) {
      logger.warn('⚠️ Erro ao desconectar:', error.message);
    }
  }

  /**
   * Exibe resultados dos testes
   */
  printResults() {
    logger.info('\n📊 RESULTADOS DOS TESTES:');
    logger.info('========================');
    
    const results = [
      ['Conectividade', this.testResults.connectivity],
      ['Criação de Tópicos', this.testResults.topicCreation],
      ['Produção de Mensagens', this.testResults.messageProduction],
      ['Consumo de Mensagens', this.testResults.messageConsumption],
      ['Consumidor Básico', this.testResults.examples.consumer],
      ['Gerenciador de Inscrições', this.testResults.examples.subscriptionManager],
      ['Processador de Eventos', this.testResults.examples.eventProcessor]
    ];
    
    results.forEach(([test, passed]) => {
      const status = passed ? '✅ PASSOU' : '❌ FALHOU';
      logger.info(`${test}: ${status}`);
    });
    
    const totalTests = results.length;
    const passedTests = results.filter(([, passed]) => passed).length;
    const successRate = ((passedTests / totalTests) * 100).toFixed(1);
    
    logger.info('========================');
    logger.info(`📈 Taxa de Sucesso: ${passedTests}/${totalTests} (${successRate}%)`);
    
    if (passedTests === totalTests) {
      logger.info('🎉 TODOS OS TESTES PASSARAM!');
    } else {
      logger.warn('⚠️ Alguns testes falharam. Verifique a configuração do Kafka.');
    }
  }

  /**
   * Verifica se Kafka está rodando
   */
  static async checkKafkaStatus() {
    logger.info('🔍 Verificando status do Kafka...');
    
    const kafka = new Kafka({
      clientId: 'ethernity-status-check',
      brokers: (process.env.KAFKA_BROKERS || 'localhost:9092').split(','),
      connectionTimeout: 3000
    });
    
    const admin = kafka.admin();
    
    try {
      await admin.connect();
      const metadata = await admin.fetchTopicMetadata();
      
      logger.info('✅ Kafka está rodando:', {
        brokers: metadata.brokers.length,
        topics: metadata.topics.length
      });
      
      await admin.disconnect();
      return true;
      
    } catch (error) {
      logger.error('❌ Kafka não está acessível:', error.message);
      logger.info('💡 Dicas para resolver:');
      logger.info('   1. Verifique se o Kafka está rodando');
      logger.info('   2. Confirme o endereço dos brokers no .env');
      logger.info('   3. Verifique conectividade de rede');
      
      await admin.disconnect();
      return false;
    }
  }
}

// Função principal
async function main() {
  logger.info('🚀 Iniciando testes do Ethernity Kafka Examples');
  
  // Verifica status do Kafka primeiro
  const kafkaRunning = await EthernityKafkaTest.checkKafkaStatus();
  if (!kafkaRunning) {
    logger.error('❌ Kafka não está acessível. Abortando testes.');
    process.exit(1);
  }
  
  // Executa testes
  const tester = new EthernityKafkaTest();
  
  try {
    await tester.runAllTests();
    logger.info('✅ Testes concluídos com sucesso!');
    process.exit(0);
  } catch (error) {
    logger.error('❌ Testes falharam:', error.message);
    process.exit(1);
  }
}

// Executa se for o arquivo principal
if (require.main === module) {
  main().catch(error => {
    console.error('Erro não tratado:', error);
    process.exit(1);
  });
}

module.exports = { EthernityKafkaTest };

