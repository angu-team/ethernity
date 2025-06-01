/**
 * Processador Avançado de Eventos Ethernity
 * 
 * Este exemplo demonstra processamento avançado de eventos,
 * incluindo análise de padrões, detecção de anomalias e
 * processamento em lotes.
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
    new winston.transports.File({ filename: 'event-processor.log' })
  ]
});

class AdvancedEventProcessor {
  constructor() {
    this.kafka = new Kafka({
      clientId: 'ethernity-event-processor',
      brokers: (process.env.KAFKA_BROKERS || 'localhost:9092').split(',')
    });
    
    this.consumer = this.kafka.consumer({ 
      groupId: 'event-processor-group',
      maxBytesPerPartition: 1024 * 1024, // 1MB
      sessionTimeout: 30000
    });
    
    this.producer = this.kafka.producer();
    
    // Armazenamento em memória para análise de padrões
    this.eventHistory = [];
    this.userActivity = new Map();
    this.tokenActivity = new Map();
    this.suspiciousPatterns = new Map();
    
    // Configurações de processamento
    this.batchSize = parseInt(process.env.BATCH_SIZE) || 100;
    this.processingTimeout = parseInt(process.env.PROCESSING_TIMEOUT_MS) || 30000;
    
    this.stats = {
      eventsProcessed: 0,
      batchesProcessed: 0,
      anomaliesDetected: 0,
      alertsSent: 0,
      startTime: null
    };
  }

  /**
   * Inicia o processador
   */
  async start() {
    try {
      logger.info('Iniciando processador avançado de eventos...');
      
      await this.consumer.connect();
      await this.producer.connect();
      
      // Inscreve-se nos tópicos
      await this.consumer.subscribe({ 
        topic: process.env.TOPIC_EVENTS || 'ethernity-events',
        fromBeginning: false 
      });

      this.stats.startTime = new Date();

      // Inicia processamento em lotes
      await this.consumer.run({
        partitionsConsumedConcurrently: 3,
        eachBatch: async ({ batch, resolveOffset, heartbeat, isRunning, isStale }) => {
          await this.processBatch(batch, resolveOffset, heartbeat, isRunning, isStale);
        }
      });

    } catch (error) {
      logger.error('Erro ao iniciar processador:', error);
      throw error;
    }
  }

  /**
   * Processa um lote de eventos
   */
  async processBatch(batch, resolveOffset, heartbeat, isRunning, isStale) {
    const { topic, partition, messages } = batch;
    
    logger.info(`Processando lote de ${messages.length} eventos`, {
      topic,
      partition,
      firstOffset: messages[0]?.offset,
      lastOffset: messages[messages.length - 1]?.offset
    });

    const events = [];
    
    // Parse das mensagens
    for (const message of messages) {
      try {
        const event = JSON.parse(message.value.toString());
        event._metadata = {
          topic,
          partition,
          offset: message.offset,
          timestamp: message.timestamp,
          key: message.key?.toString()
        };
        events.push(event);
      } catch (error) {
        logger.error('Erro ao fazer parse do evento:', {
          error: error.message,
          offset: message.offset
        });
      }
    }

    // Processa eventos em paralelo
    const processingPromises = events.map(event => this.processEvent(event));
    await Promise.allSettled(processingPromises);

    // Análise de padrões no lote
    await this.analyzeBatchPatterns(events);

    // Detecção de anomalias
    await this.detectAnomalies(events);

    // Confirma processamento
    for (const message of messages) {
      resolveOffset(message.offset);
      await heartbeat();
    }

    this.stats.batchesProcessed++;
    this.stats.eventsProcessed += events.length;

    // Limpa histórico antigo
    this.cleanupHistory();
  }

  /**
   * Processa um evento individual
   */
  async processEvent(event) {
    try {
      // Adiciona ao histórico
      this.eventHistory.push({
        ...event,
        processedAt: new Date()
      });

      // Atualiza atividade do usuário
      if (event.data?.user) {
        this.updateUserActivity(event.data.user, event);
      }

      // Atualiza atividade do token
      if (event.data?.token?.address) {
        this.updateTokenActivity(event.data.token.address, event);
      }

      // Processamento específico por tipo
      await this.processEventByType(event);

    } catch (error) {
      logger.error('Erro ao processar evento:', {
        error: error.message,
        eventType: event.event_type,
        offset: event._metadata?.offset
      });
    }
  }

  /**
   * Processa evento baseado no tipo
   */
  async processEventByType(event) {
    switch (event.event_type) {
      case 'large_transfer':
        await this.processLargeTransfer(event);
        break;
      
      case 'token_swap':
        await this.processTokenSwap(event);
        break;
      
      case 'liquidation':
        await this.processLiquidation(event);
        break;
      
      case 'mev_activity':
        await this.processMevActivity(event);
        break;
      
      case 'flash_loan':
        await this.processFlashLoan(event);
        break;
      
      default:
        logger.debug(`Processamento padrão para evento: ${event.event_type}`);
    }
  }

  /**
   * Processa transferências grandes
   */
  async processLargeTransfer(event) {
    const { from, to, amount, usd_value, token } = event.data;
    
    // Verifica se é uma transferência suspeita
    if (usd_value > 1000000) { // > $1M
      await this.flagSuspiciousActivity('large_transfer', {
        from,
        to,
        amount: usd_value,
        token: token.symbol,
        reason: 'Transferência acima de $1M'
      });
    }

    // Analisa padrão de transferências do usuário
    const userTransfers = this.getUserRecentActivity(from, 'large_transfer');
    if (userTransfers.length > 5) { // Mais de 5 transferências grandes recentes
      await this.flagSuspiciousActivity('frequent_large_transfers', {
        user: from,
        count: userTransfers.length,
        totalValue: userTransfers.reduce((sum, t) => sum + t.data.usd_value, 0)
      });
    }
  }

  /**
   * Processa swaps de tokens
   */
  async processTokenSwap(event) {
    const { user, token_in, token_out, amount_in, amount_out, dex_protocol } = event.data;
    
    // Detecta possível front-running
    const recentSwaps = this.getUserRecentActivity(user, 'token_swap', 60000); // Últimos 60s
    const sameTokenPairSwaps = recentSwaps.filter(swap => 
      (swap.data.token_in.address === token_in.address && 
       swap.data.token_out.address === token_out.address) ||
      (swap.data.token_in.address === token_out.address && 
       swap.data.token_out.address === token_in.address)
    );

    if (sameTokenPairSwaps.length > 3) {
      await this.flagSuspiciousActivity('possible_front_running', {
        user,
        tokenPair: `${token_in.symbol}/${token_out.symbol}`,
        swapCount: sameTokenPairSwaps.length,
        timeWindow: '60s'
      });
    }

    // Analisa impacto no preço
    await this.analyzePriceImpact(event);
  }

  /**
   * Processa liquidações
   */
  async processLiquidation(event) {
    const { liquidated_user, liquidator, liquidated_amount } = event.data;
    
    // Verifica se o liquidador está fazendo muitas liquidações
    const liquidatorActivity = this.getUserRecentActivity(liquidator, 'liquidation');
    if (liquidatorActivity.length > 10) {
      await this.flagSuspiciousActivity('aggressive_liquidator', {
        liquidator,
        liquidationCount: liquidatorActivity.length,
        totalLiquidated: liquidatorActivity.reduce((sum, l) => sum + l.data.liquidated_amount, 0)
      });
    }
  }

  /**
   * Processa atividade MEV
   */
  async processMevActivity(event) {
    const { bot_address, profit_usd, mev_type } = event.data;
    
    // Rastreia bots MEV mais ativos
    const botActivity = this.getUserRecentActivity(bot_address, 'mev_activity');
    const totalProfit = botActivity.reduce((sum, activity) => sum + activity.data.profit_usd, 0);
    
    if (totalProfit > 100000) { // > $100k em lucros MEV
      await this.sendAlert('high_mev_profit', {
        bot: bot_address,
        totalProfit,
        activityCount: botActivity.length,
        mevType: mev_type
      });
    }
  }

  /**
   * Processa flash loans
   */
  async processFlashLoan(event) {
    const { user, amount, token, protocol } = event.data;
    
    // Detecta uso frequente de flash loans (possível ataque)
    const userFlashLoans = this.getUserRecentActivity(user, 'flash_loan');
    if (userFlashLoans.length > 3) {
      await this.flagSuspiciousActivity('frequent_flash_loans', {
        user,
        count: userFlashLoans.length,
        protocols: [...new Set(userFlashLoans.map(fl => fl.data.protocol))],
        totalAmount: userFlashLoans.reduce((sum, fl) => sum + fl.data.amount, 0)
      });
    }
  }

  /**
   * Analisa padrões em um lote de eventos
   */
  async analyzeBatchPatterns(events) {
    // Agrupa eventos por tipo
    const eventsByType = events.reduce((acc, event) => {
      acc[event.event_type] = acc[event.event_type] || [];
      acc[event.event_type].push(event);
      return acc;
    }, {});

    // Detecta picos de atividade
    for (const [eventType, typeEvents] of Object.entries(eventsByType)) {
      if (typeEvents.length > 50) { // Pico de atividade
        await this.sendAlert('activity_spike', {
          eventType,
          count: typeEvents.length,
          timeWindow: 'batch'
        });
      }
    }

    // Analisa correlações entre eventos
    await this.analyzeEventCorrelations(events);
  }

  /**
   * Analisa correlações entre eventos
   */
  async analyzeEventCorrelations(events) {
    // Procura por padrões de sandwich attacks
    const swaps = events.filter(e => e.event_type === 'token_swap');
    const mevActivities = events.filter(e => e.event_type === 'mev_activity');
    
    for (const swap of swaps) {
      const relatedMev = mevActivities.find(mev => 
        mev.data.mev_type === 'sandwich_attack' &&
        Math.abs(new Date(mev.timestamp) - new Date(swap.timestamp)) < 30000 // 30s
      );
      
      if (relatedMev) {
        await this.flagSuspiciousActivity('sandwich_attack_detected', {
          victim: swap.data.user,
          attacker: relatedMev.data.bot_address,
          tokenPair: `${swap.data.token_in.symbol}/${swap.data.token_out.symbol}`,
          profit: relatedMev.data.profit_usd
        });
      }
    }
  }

  /**
   * Detecta anomalias nos eventos
   */
  async detectAnomalies(events) {
    // Detecta volumes anômalos
    const transfers = events.filter(e => e.event_type === 'large_transfer');
    const avgValue = transfers.reduce((sum, t) => sum + t.data.usd_value, 0) / transfers.length;
    
    for (const transfer of transfers) {
      if (transfer.data.usd_value > avgValue * 10) { // 10x acima da média
        await this.flagSuspiciousActivity('anomalous_transfer_volume', {
          transfer: transfer.data,
          averageValue: avgValue,
          multiplier: transfer.data.usd_value / avgValue
        });
      }
    }

    this.stats.anomaliesDetected += this.suspiciousPatterns.size;
  }

  /**
   * Analisa impacto no preço
   */
  async analyzePriceImpact(swapEvent) {
    const { amount_in, amount_out, token_in, token_out } = swapEvent.data;
    
    // Calcula impacto aproximado no preço (simplificado)
    const priceImpact = Math.abs(1 - (amount_out / amount_in));
    
    if (priceImpact > 0.05) { // > 5% de impacto
      await this.sendAlert('high_price_impact', {
        user: swapEvent.data.user,
        tokenPair: `${token_in.symbol}/${token_out.symbol}`,
        priceImpact: (priceImpact * 100).toFixed(2) + '%',
        amountIn: amount_in,
        amountOut: amount_out
      });
    }
  }

  /**
   * Atualiza atividade do usuário
   */
  updateUserActivity(userAddress, event) {
    if (!this.userActivity.has(userAddress)) {
      this.userActivity.set(userAddress, []);
    }
    
    const userEvents = this.userActivity.get(userAddress);
    userEvents.push(event);
    
    // Mantém apenas os últimos 100 eventos por usuário
    if (userEvents.length > 100) {
      userEvents.splice(0, userEvents.length - 100);
    }
  }

  /**
   * Atualiza atividade do token
   */
  updateTokenActivity(tokenAddress, event) {
    if (!this.tokenActivity.has(tokenAddress)) {
      this.tokenActivity.set(tokenAddress, []);
    }
    
    const tokenEvents = this.tokenActivity.get(tokenAddress);
    tokenEvents.push(event);
    
    // Mantém apenas os últimos 50 eventos por token
    if (tokenEvents.length > 50) {
      tokenEvents.splice(0, tokenEvents.length - 50);
    }
  }

  /**
   * Obtém atividade recente do usuário
   */
  getUserRecentActivity(userAddress, eventType = null, timeWindowMs = 3600000) { // 1 hora
    const userEvents = this.userActivity.get(userAddress) || [];
    const cutoffTime = Date.now() - timeWindowMs;
    
    return userEvents.filter(event => {
      const eventTime = new Date(event.timestamp).getTime();
      const isRecent = eventTime > cutoffTime;
      const isCorrectType = !eventType || event.event_type === eventType;
      return isRecent && isCorrectType;
    });
  }

  /**
   * Marca atividade suspeita
   */
  async flagSuspiciousActivity(patternType, data) {
    const key = `${patternType}_${JSON.stringify(data).substring(0, 50)}`;
    
    if (!this.suspiciousPatterns.has(key)) {
      this.suspiciousPatterns.set(key, {
        type: patternType,
        data,
        firstSeen: new Date(),
        count: 1
      });
      
      logger.warn(`🚨 Atividade suspeita detectada: ${patternType}`, data);
      await this.sendAlert('suspicious_activity', { patternType, data });
    } else {
      const pattern = this.suspiciousPatterns.get(key);
      pattern.count++;
      pattern.lastSeen = new Date();
    }
  }

  /**
   * Envia alerta
   */
  async sendAlert(alertType, data) {
    try {
      const alert = {
        type: alertType,
        timestamp: new Date().toISOString(),
        severity: this.getAlertSeverity(alertType),
        data
      };

      await this.producer.send({
        topic: 'ethernity-alerts',
        messages: [{
          key: alertType,
          value: JSON.stringify(alert)
        }]
      });

      this.stats.alertsSent++;
      logger.info(`📢 Alerta enviado: ${alertType}`, data);

    } catch (error) {
      logger.error('Erro ao enviar alerta:', error);
    }
  }

  /**
   * Determina severidade do alerta
   */
  getAlertSeverity(alertType) {
    const severityMap = {
      'sandwich_attack_detected': 'high',
      'anomalous_transfer_volume': 'high',
      'frequent_flash_loans': 'medium',
      'aggressive_liquidator': 'medium',
      'high_mev_profit': 'low',
      'activity_spike': 'low',
      'high_price_impact': 'medium',
      'suspicious_activity': 'medium'
    };
    
    return severityMap[alertType] || 'low';
  }

  /**
   * Limpa histórico antigo
   */
  cleanupHistory() {
    const cutoffTime = Date.now() - (24 * 60 * 60 * 1000); // 24 horas
    
    // Limpa histórico de eventos
    this.eventHistory = this.eventHistory.filter(event => 
      new Date(event.processedAt).getTime() > cutoffTime
    );

    // Limpa padrões suspeitos antigos
    for (const [key, pattern] of this.suspiciousPatterns.entries()) {
      if (new Date(pattern.firstSeen).getTime() < cutoffTime) {
        this.suspiciousPatterns.delete(key);
      }
    }
  }

  /**
   * Para o processador
   */
  async stop() {
    logger.info('Parando processador de eventos...');
    
    await this.consumer.disconnect();
    await this.producer.disconnect();
    
    this.printStats();
    logger.info('Processador parado');
  }

  /**
   * Exibe estatísticas
   */
  printStats() {
    const uptime = this.stats.startTime ? 
      Math.round((Date.now() - this.stats.startTime.getTime()) / 1000) : 0;
    
    logger.info('Estatísticas do processador:', {
      ...this.stats,
      uptimeSeconds: uptime,
      eventsPerSecond: uptime > 0 ? (this.stats.eventsProcessed / uptime).toFixed(2) : 0,
      activeUsers: this.userActivity.size,
      activeTokens: this.tokenActivity.size,
      suspiciousPatterns: this.suspiciousPatterns.size
    });
  }

  /**
   * Obtém estatísticas detalhadas
   */
  getDetailedStats() {
    return {
      processing: { ...this.stats },
      memory: {
        eventHistorySize: this.eventHistory.length,
        userActivitySize: this.userActivity.size,
        tokenActivitySize: this.tokenActivity.size,
        suspiciousPatternsSize: this.suspiciousPatterns.size
      },
      patterns: Array.from(this.suspiciousPatterns.values())
    };
  }
}

// Função principal
async function main() {
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

  // Exibe estatísticas periodicamente
  setInterval(() => {
    processor.printStats();
  }, 60000); // A cada minuto

  // Inicia o processador
  try {
    await processor.start();
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

module.exports = { AdvancedEventProcessor };

