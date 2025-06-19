# ethernity-finder

Ferramenta para localizar nodes Ethereum utilizando a API do Shodan.

O objetivo é buscar endpoints RPC expostos, validar o `chainId` e verificar se
métodos RPC internos específicos estão disponíveis.

Os métodos aceitos são representados pelo enum `RpcMethod`, que inclui:

- `debug_traceTransaction`
- `admin_nodeInfo`
- `admin_peers`
- `txpool_content`
- `trace_block`

A crate expõe um trait `NodeFinder` com implementação padrão `ShodanFinder` que
pode ser reutilizada pelos demais módulos do projeto.

Consulte o diretório [`examples`](./examples/) para um exemplo de uso via linha
de comando.
