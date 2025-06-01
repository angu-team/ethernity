/**
 * Consumidor básico de eventos Kafka do Ethernity
 * 
 * Este exemplo demonstra como se inscrever em tópicos Kafka
 * e processar eventos blockchain em tempo real.
 */

const { Kafka } = require('kafkajs');
const winston = require('winston');
require('dotenv').config();

// Configuração do logger
const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.errors({ stack: true }),
    winston.format.json()
  ),
  transports: [
    new winston.transports.Console({
      format: winston.format.combine(
        winston.format.colorize(),
        winston.format.simple()
      )
    }),
    new winston.transports.File({ filename: 'ethernity-consumer.log' })
  ]
});

// Configuração do Kafka
const kafka = new Kafka({
  clientId: process.env.KAFKA_CLIENT_ID || 'ethernity-nodejs-consumer',
  brokers: (process.env.KAFKA_BROKERS || 'localhost:9092').split(','),
  retry: {
    initialRetryTime: 100,
    retries: parseInt(process.env.MAX_RETRIES) || 3
  }
});

class EthernityEventConsumer {
  constructor() {
    this.consumer = kafka.consumer({ 
      groupId: process.env.KAFKA_GROUP_ID || 'ethernity-events-group',
      sessionTimeout: 30000,
      heartbeatInterval: 3000
    });
    this.isRunning = false;
    this.eventHandlers = new Map();
    this.stats = {
      messagesProcessed: 0,
      errors: 0,
      startTime: null
    };
  }

  /**
   * Registra um handler para um tipo específico de evento
   */
  registerEventHandler(eventType, handler) {
    if (typeof handler !== 'function') {
      throw new Error('Handler deve ser uma função');
    }
    this.eventHandlers.set(eventType, handler);
    logger.info(`Handler registrado para evento: ${eventType}`);
  }

  /**
   * Inicia o consumidor
   */
  async start() {
    try {
      logger.info('Iniciando consumidor Ethernity...');
      
      await this.consumer.connect();
      logger.info('Conectado ao Kafka');

      // Inscreve-se nos tópicos
      const topics = [
        process.env.TOPIC_EVENTS || 'ethernity-events',
        process.env.TOPIC_NOTIFICATIONS || 'ethernity-notifications'
      ];

      for (const topic of topics) {
        await this.consumer.subscribe({ topic, fromBeginning: false });
        logger.info(`Inscrito no tópico: ${topic}`);
      }

      this.isRunning = true;
      this.stats.startTime = new Date();

      // Inicia o loop de processamento
      await this.consumer.run({
        eachMessage: async ({ topic, partition, message }) => {
          await this.processMessage(topic, partition, message);
        },
        eachBatch: async ({ batch }) => {
          await this.processBatch(batch);
        }
      });

    } catch (error) {
      logger.error('Erro ao iniciar consumidor:', error);
      throw error;
    }
  }

  /**
   * Processa uma mensagem individual
   */
  async processMessage(topic, partition, message) {
    try {
      const messageValue = message.value.toString();
      const event = JSON.parse(messageValue);
      
      logger.debug(`Processando mensagem do tópico ${topic}:`, {
        partition,
        offset: message.offset,
        eventType: event.event_type
      });

      // Processa baseado no tipo de evento
      await this.handleEvent(event, topic);
      
      this.stats.messagesProcessed++;

    } catch (error) {
      this.stats.errors++;
      logger.error('Erro ao processar mensagem:', {
        error: error.message,
        topic,
        partition,
        offset: message.offset
      });
    }
  }

  /**
   * Processa um lote de mensagens
   */
  async processBatch(batch) {
    const { topic, partition, messages } = batch;
    
    logger.info(`Processando lote de ${messages.length} mensagens do tópico ${topic}`);
    
    for (const message of messages) {
      await this.processMessage(topic, partition, message);
    }
  }

  /**
   * Manipula um evento específico
   */
  async handleEvent(event, topic) {
    const eventType = event.event_type || event.type;
    
    // Verifica se há um handler específico registrado
    if (this.eventHandlers.has(eventType)) {
      const handler = this.eventHandlers.get(eventType);
      await handler(event, topic);
      return;
    }

    // Handler padrão baseado no tipo de evento
    switch (eventType) {
      case 'erc20_created':
        await this.handleErc20Created(event);
        break;
      
      case 'token_swap':
        await this.handleTokenSwap(event);
        break;
      
      case 'large_transfer':
        await this.handleLargeTransfer(event);
        break;
      
      case 'liquidation':
        await this.handleLiquidation(event);
        break;
      
      case 'rug_pull_warning':
        await this.handleRugPullWarning(event);
        break;
      
      case 'mev_activity':
        await this.handleMevActivity(event);
        break;
      
      case 'flash_loan':
        await this.handleFlashLoan(event);
        break;
      
      case 'governance_event':
        await this.handleGovernanceEvent(event);
        break;
      
      default:
        logger.warn(`Tipo de evento não reconhecido: ${eventType}`);
        await this.handleUnknownEvent(event);
    }
  }

  /**
   * Handlers específicos para cada tipo de evento
   */
  async handleErc20Created(event) {
    logger.info('🪙 Novo token ERC20 criado:', {
      contractAddress: event.data.contract_address,
      creator: event.data.creator,
      name: event.data.name,
      symbol: event.data.symbol,
      totalSupply: event.data.total_supply
    });
  }

  async handleTokenSwap(event) {
    logger.info('🔄 Swap de token detectado:', {
      user: event.data.user,
      tokenIn: event.data.token_in.symbol,
      tokenOut: event.data.token_out.symbol,
      amountIn: event.data.amount_in,
      amountOut: event.data.amount_out,
      dex: event.data.dex_protocol
    });
  }

  async handleLargeTransfer(event) {
    logger.info('💰 Grande transferência detectada:', {
      from: event.data.from,
      to: event.data.to,
      token: event.data.token.symbol,
      amount: event.data.amount,
      usdValue: event.data.usd_value
    });
  }

  async handleLiquidation(event) {
    logger.warn('⚠️ Liquidação detectada:', {
      liquidatedUser: event.data.liquidated_user,
      liquidator: event.data.liquidator,
      collateralToken: event.data.collateral_token.symbol,
      debtToken: event.data.debt_token.symbol,
      liquidatedAmount: event.data.liquidated_amount
    });
  }

  async handleRugPullWarning(event) {
    logger.error('🚨 ALERTA: Possível rug pull detectado:', {
      token: event.data.token.symbol,
      contractAddress: event.data.token.address,
      deployer: event.data.deployer,
      riskScore: event.data.risk_score,
      indicators: event.data.risk_indicators
    });
  }

  async handleMevActivity(event) {
    logger.info('🤖 Atividade MEV detectada:', {
      type: event.data.mev_type,
      bot: event.data.bot_address,
      profit: event.data.profit_usd,
      gasUsed: event.data.gas_used
    });
  }

  async handleFlashLoan(event) {
    logger.info('⚡ Flash loan detectado:', {
      user: event.data.user,
      token: event.data.token.symbol,
      amount: event.data.amount,
      fee: event.data.fee,
      protocol: event.data.protocol
    });
  }

  async handleGovernanceEvent(event) {
    logger.info('🏛️ Evento de governança:', {
      type: event.data.governance_type,
      proposalId: event.data.proposal_id,
      proposer: event.data.proposer,
      description: event.data.description
    });
  }

  async handleUnknownEvent(event) {
    logger.info('❓ Evento desconhecido:', {
      type: event.event_type || event.type,
      data: event.data
    });
  }

  /**
   * Para o consumidor graciosamente
   */
  async stop() {
    if (!this.isRunning) {
      return;
    }

    logger.info('Parando consumidor...');
    this.isRunning = false;
    
    await this.consumer.disconnect();
    logger.info('Consumidor parado');
    
    this.printStats();
  }

  /**
   * Exibe estatísticas do consumidor
   */
  printStats() {
    const uptime = this.stats.startTime ? 
      Math.round((Date.now() - this.stats.startTime.getTime()) / 1000) : 0;
    
    logger.info('Estatísticas do consumidor:', {
      messagesProcessed: this.stats.messagesProcessed,
      errors: this.stats.errors,
      uptimeSeconds: uptime,
      messagesPerSecond: uptime > 0 ? (this.stats.messagesProcessed / uptime).toFixed(2) : 0
    });
  }

  /**
   * Obtém estatísticas atuais
   */
  getStats() {
    return { ...this.stats };
  }
}

// Função principal
async function main() {
  const consumer = new EthernityEventConsumer();

  // Registra handlers personalizados (exemplo)
  consumer.registerEventHandler('custom_event', async (event, topic) => {
    logger.info('Handler personalizado executado:', event);
  });

  // Manipula sinais de sistema para parada graciosa
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

  // Inicia o consumidor
  try {
    await consumer.start();
  } catch (error) {
    logger.error('Erro fatal:', error);
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

module.exports = { EthernityEventConsumer };

