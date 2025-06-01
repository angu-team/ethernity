/**
 * Gerenciador de Inscrições Ethernity
 * 
 * Este exemplo demonstra como gerenciar inscrições de eventos
 * dinamicamente através de comandos Kafka.
 */

const { Kafka } = require('kafkajs');
const winston = require('winston');
const { v4: uuidv4 } = require('uuid');
const Joi = require('joi');
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
    new winston.transports.File({ filename: 'subscription-manager.log' })
  ]
});

// Schema de validação para inscrições
const subscriptionSchema = Joi.object({
  id: Joi.string().optional(),
  user_id: Joi.string().required(),
  event_type: Joi.string().valid(
    'erc20_created',
    'token_swap',
    'large_transfer',
    'liquidation',
    'rug_pull_warning',
    'mev_activity',
    'flash_loan',
    'governance_event'
  ).required(),
  filters: Joi.object({
    general: Joi.object({
      min_value_usd: Joi.number().min(0).optional(),
      max_value_usd: Joi.number().min(0).optional(),
      include_mempool: Joi.boolean().default(true),
      include_confirmed: Joi.boolean().default(true),
      min_confirmations: Joi.number().min(0).default(0),
      address_whitelist: Joi.array().items(Joi.string()).default([]),
      address_blacklist: Joi.array().items(Joi.string()).default([])
    }).default({}),
    specific: Joi.object().default({})
  }).default({}),
  notification_config: Joi.object({
    method: Joi.string().valid('webhook', 'websocket', 'kafka', 'email', 'sms').required(),
    webhook_url: Joi.string().uri().when('method', { is: 'webhook', then: Joi.required() }),
    websocket_connection_id: Joi.string().when('method', { is: 'websocket', then: Joi.required() }),
    kafka_topic: Joi.string().when('method', { is: 'kafka', then: Joi.required() }),
    email_address: Joi.string().email().when('method', { is: 'email', then: Joi.required() }),
    phone_number: Joi.string().when('method', { is: 'sms', then: Joi.required() }),
    retry_policy: Joi.object({
      max_retries: Joi.number().min(0).default(3),
      initial_delay: Joi.number().min(0).default(1000),
      max_delay: Joi.number().min(0).default(30000),
      backoff_multiplier: Joi.number().min(1).default(2)
    }).default({})
  }).required(),
  rate_limit: Joi.object({
    events_per_minute: Joi.number().min(1).default(60),
    events_per_hour: Joi.number().min(1).default(1000),
    events_per_day: Joi.number().min(1).default(10000),
    burst_allowance: Joi.number().min(1).default(10),
    webhook_timeout: Joi.number().min(1000).default(30000)
  }).optional(),
  expires_at: Joi.date().optional(),
  is_active: Joi.boolean().default(true)
});

class SubscriptionManager {
  constructor() {
    this.kafka = new Kafka({
      clientId: process.env.KAFKA_CLIENT_ID || 'ethernity-subscription-manager',
      brokers: (process.env.KAFKA_BROKERS || 'localhost:9092').split(',')
    });
    
    this.producer = this.kafka.producer();
    this.consumer = this.kafka.consumer({ 
      groupId: 'subscription-manager-group' 
    });
    
    this.subscriptions = new Map();
    this.isRunning = false;
  }

  /**
   * Inicia o gerenciador
   */
  async start() {
    try {
      logger.info('Iniciando gerenciador de inscrições...');
      
      await this.producer.connect();
      await this.consumer.connect();
      
      // Inscreve-se no tópico de comandos de inscrição
      await this.consumer.subscribe({ 
        topic: process.env.TOPIC_SUBSCRIPTIONS || 'ethernity-subscriptions',
        fromBeginning: false 
      });

      this.isRunning = true;

      // Inicia o loop de processamento de comandos
      await this.consumer.run({
        eachMessage: async ({ topic, partition, message }) => {
          await this.processSubscriptionCommand(message);
        }
      });

    } catch (error) {
      logger.error('Erro ao iniciar gerenciador:', error);
      throw error;
    }
  }

  /**
   * Processa comandos de inscrição
   */
  async processSubscriptionCommand(message) {
    try {
      const command = JSON.parse(message.value.toString());
      
      logger.debug('Processando comando de inscrição:', {
        command: command.type,
        subscriptionId: command.subscription_id
      });

      switch (command.type) {
        case 'create':
          await this.createSubscription(command.data);
          break;
        
        case 'update':
          await this.updateSubscription(command.subscription_id, command.data);
          break;
        
        case 'delete':
          await this.deleteSubscription(command.subscription_id);
          break;
        
        case 'get':
          await this.getSubscription(command.subscription_id);
          break;
        
        case 'list':
          await this.listSubscriptions(command.user_id);
          break;
        
        default:
          logger.warn(`Comando desconhecido: ${command.type}`);
      }

    } catch (error) {
      logger.error('Erro ao processar comando de inscrição:', error);
    }
  }

  /**
   * Cria uma nova inscrição
   */
  async createSubscription(subscriptionData) {
    try {
      // Valida os dados da inscrição
      const { error, value } = subscriptionSchema.validate(subscriptionData);
      if (error) {
        throw new Error(`Dados de inscrição inválidos: ${error.message}`);
      }

      // Gera ID se não fornecido
      if (!value.id) {
        value.id = uuidv4();
      }

      // Adiciona timestamps
      value.created_at = new Date().toISOString();
      value.updated_at = value.created_at;

      // Armazena a inscrição
      this.subscriptions.set(value.id, value);

      logger.info('Inscrição criada:', {
        id: value.id,
        userId: value.user_id,
        eventType: value.event_type,
        notificationMethod: value.notification_config.method
      });

      // Envia confirmação
      await this.sendResponse('subscription_created', {
        subscription_id: value.id,
        subscription: value
      });

      return value;

    } catch (error) {
      logger.error('Erro ao criar inscrição:', error);
      await this.sendResponse('subscription_error', {
        error: error.message,
        type: 'create'
      });
      throw error;
    }
  }

  /**
   * Atualiza uma inscrição existente
   */
  async updateSubscription(subscriptionId, updateData) {
    try {
      if (!this.subscriptions.has(subscriptionId)) {
        throw new Error(`Inscrição ${subscriptionId} não encontrada`);
      }

      const existingSubscription = this.subscriptions.get(subscriptionId);
      const updatedData = { ...existingSubscription, ...updateData };
      updatedData.updated_at = new Date().toISOString();

      // Valida os dados atualizados
      const { error, value } = subscriptionSchema.validate(updatedData);
      if (error) {
        throw new Error(`Dados de atualização inválidos: ${error.message}`);
      }

      // Atualiza a inscrição
      this.subscriptions.set(subscriptionId, value);

      logger.info('Inscrição atualizada:', {
        id: subscriptionId,
        changes: Object.keys(updateData)
      });

      // Envia confirmação
      await this.sendResponse('subscription_updated', {
        subscription_id: subscriptionId,
        subscription: value
      });

      return value;

    } catch (error) {
      logger.error('Erro ao atualizar inscrição:', error);
      await this.sendResponse('subscription_error', {
        subscription_id: subscriptionId,
        error: error.message,
        type: 'update'
      });
      throw error;
    }
  }

  /**
   * Remove uma inscrição
   */
  async deleteSubscription(subscriptionId) {
    try {
      if (!this.subscriptions.has(subscriptionId)) {
        throw new Error(`Inscrição ${subscriptionId} não encontrada`);
      }

      const subscription = this.subscriptions.get(subscriptionId);
      this.subscriptions.delete(subscriptionId);

      logger.info('Inscrição removida:', {
        id: subscriptionId,
        userId: subscription.user_id
      });

      // Envia confirmação
      await this.sendResponse('subscription_deleted', {
        subscription_id: subscriptionId
      });

    } catch (error) {
      logger.error('Erro ao remover inscrição:', error);
      await this.sendResponse('subscription_error', {
        subscription_id: subscriptionId,
        error: error.message,
        type: 'delete'
      });
      throw error;
    }
  }

  /**
   * Obtém uma inscrição específica
   */
  async getSubscription(subscriptionId) {
    try {
      if (!this.subscriptions.has(subscriptionId)) {
        throw new Error(`Inscrição ${subscriptionId} não encontrada`);
      }

      const subscription = this.subscriptions.get(subscriptionId);

      // Envia resposta
      await this.sendResponse('subscription_found', {
        subscription_id: subscriptionId,
        subscription
      });

      return subscription;

    } catch (error) {
      logger.error('Erro ao obter inscrição:', error);
      await this.sendResponse('subscription_error', {
        subscription_id: subscriptionId,
        error: error.message,
        type: 'get'
      });
      throw error;
    }
  }

  /**
   * Lista inscrições de um usuário
   */
  async listSubscriptions(userId) {
    try {
      const userSubscriptions = Array.from(this.subscriptions.values())
        .filter(sub => sub.user_id === userId);

      logger.info(`Listando ${userSubscriptions.length} inscrições para usuário ${userId}`);

      // Envia resposta
      await this.sendResponse('subscriptions_listed', {
        user_id: userId,
        subscriptions: userSubscriptions,
        count: userSubscriptions.length
      });

      return userSubscriptions;

    } catch (error) {
      logger.error('Erro ao listar inscrições:', error);
      await this.sendResponse('subscription_error', {
        user_id: userId,
        error: error.message,
        type: 'list'
      });
      throw error;
    }
  }

  /**
   * Envia resposta via Kafka
   */
  async sendResponse(type, data) {
    try {
      const response = {
        type,
        timestamp: new Date().toISOString(),
        data
      };

      await this.producer.send({
        topic: 'ethernity-subscription-responses',
        messages: [{
          key: data.subscription_id || data.user_id || 'system',
          value: JSON.stringify(response)
        }]
      });

    } catch (error) {
      logger.error('Erro ao enviar resposta:', error);
    }
  }

  /**
   * Para o gerenciador
   */
  async stop() {
    if (!this.isRunning) {
      return;
    }

    logger.info('Parando gerenciador de inscrições...');
    this.isRunning = false;
    
    await this.producer.disconnect();
    await this.consumer.disconnect();
    
    logger.info('Gerenciador parado');
  }

  /**
   * Obtém estatísticas
   */
  getStats() {
    const subscriptionsByType = {};
    const subscriptionsByMethod = {};
    let activeSubscriptions = 0;

    for (const subscription of this.subscriptions.values()) {
      // Por tipo de evento
      subscriptionsByType[subscription.event_type] = 
        (subscriptionsByType[subscription.event_type] || 0) + 1;
      
      // Por método de notificação
      subscriptionsByMethod[subscription.notification_config.method] = 
        (subscriptionsByMethod[subscription.notification_config.method] || 0) + 1;
      
      // Ativas
      if (subscription.is_active) {
        activeSubscriptions++;
      }
    }

    return {
      totalSubscriptions: this.subscriptions.size,
      activeSubscriptions,
      subscriptionsByType,
      subscriptionsByMethod
    };
  }
}

// Função utilitária para criar comandos de inscrição
function createSubscriptionCommand(type, data) {
  return {
    type,
    timestamp: new Date().toISOString(),
    ...data
  };
}

// Exemplos de uso
async function examples() {
  const manager = new SubscriptionManager();
  
  // Exemplo de criação de inscrição
  const createCommand = createSubscriptionCommand('create', {
    data: {
      user_id: 'user123',
      event_type: 'large_transfer',
      filters: {
        general: {
          min_value_usd: 10000,
          include_mempool: true
        }
      },
      notification_config: {
        method: 'webhook',
        webhook_url: 'https://api.example.com/webhooks/ethernity'
      }
    }
  });

  // Exemplo de atualização
  const updateCommand = createSubscriptionCommand('update', {
    subscription_id: 'sub-123',
    data: {
      filters: {
        general: {
          min_value_usd: 50000
        }
      }
    }
  });

  // Exemplo de remoção
  const deleteCommand = createSubscriptionCommand('delete', {
    subscription_id: 'sub-123'
  });

  logger.info('Exemplos de comandos criados:', {
    create: createCommand,
    update: updateCommand,
    delete: deleteCommand
  });
}

// Função principal
async function main() {
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
    logger.info('Estatísticas do gerenciador:', stats);
  }, 30000);

  // Inicia o gerenciador
  try {
    await manager.start();
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

module.exports = { 
  SubscriptionManager, 
  createSubscriptionCommand,
  subscriptionSchema 
};

