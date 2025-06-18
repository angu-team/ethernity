# ethernity-finder

Ferramenta para localizar nodes Ethereum utilizando a API do Shodan.

O objetivo é buscar endpoints RPC expostos, validar o `chainId` e verificar se
métodos RPC internos específicos estão disponíveis.

A crate expõe um trait `NodeFinder` com implementação padrão `ShodanFinder` que
pode ser reutilizada pelos demais módulos do projeto.

Consulte o diretório [`examples`](./examples/) para um exemplo de uso via linha
de comando.
