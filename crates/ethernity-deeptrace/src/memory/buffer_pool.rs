use parking_lot::RwLock;

/// Pool de buffers para reutilização
pub struct BufferPool {
    buffers: RwLock<Vec<Vec<u8>>>,
    buffer_size: usize,
    max_buffers: usize,
    stats: RwLock<BufferPoolStats>,
}

/// Estatísticas do pool de buffers
#[derive(Debug, Default, Clone)]
pub struct BufferPoolStats {
    pub allocations: usize,
    pub reuses: usize,
    pub returns: usize,
    pub misses: usize,
}

impl BufferPool {
    /// Cria um novo pool de buffers
    pub fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffers: RwLock::new(Vec::with_capacity(max_buffers)),
            buffer_size,
            max_buffers,
            stats: RwLock::new(BufferPoolStats::default()),
        }
    }

    /// Obtém um buffer do pool ou cria um novo
    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buffers = self.buffers.write();

        if let Some(mut buffer) = buffers.pop() {
            // Limpa o buffer antes de reutilizar
            buffer.clear();
            self.stats.write().reuses += 1;
            buffer
        } else {
            // Cria um novo buffer
            self.stats.write().allocations += 1;
            self.stats.write().misses += 1;
            Vec::with_capacity(self.buffer_size)
        }
    }

    /// Devolve um buffer ao pool
    pub fn return_buffer(&self, mut buffer: Vec<u8>) {
        let mut buffers = self.buffers.write();

        // Só adiciona ao pool se houver espaço
        if buffers.len() < self.max_buffers {
            // Limpa o buffer antes de devolver
            buffer.clear();
            buffers.push(buffer);
            self.stats.write().returns += 1;
        }
    }

    /// Obtém estatísticas do pool
    pub fn stats(&self) -> BufferPoolStats {
        self.stats.read().clone()
    }
}

