# La TUI corre en el PC y un Agent en el Remote hace el trabajo sobre disco

Para hacer Transfers de Groups desde el servidor al PC, ratatidy tiene que poder correr en el PC. La detección de hardlinks depende de `(dev, inode)`, que solo se puede leer de forma fiable en la propia máquina. Por eso un **Agent** (el mismo binario de ratatidy, en el Remote) escanea, borra y habla con qBittorrent en `localhost`. La TUI lo invoca con comandos one-shot por SSH (`ssh <alias> ratatidy agent <comando>` → JSON) y comprueba una versión de protocolo que tiene que coincidir. La configuración de rutas y las credenciales de qBittorrent se quedan en el Remote. El PC solo conoce el alias SSH del Remote. Sin Remote configurado, ratatidy sigue funcionando en local como antes.

## Considered Options

- **`ssh host find -printf`**: no requiere instalar nada en el servidor, pero duplica la lógica del scanner, pierde su caché y deja el borrado y qBittorrent sin resolver.
- **SFTP nativo**: un viaje de ida y vuelta por fichero; demasiado lento con bibliotecas de varios TB.
- **Sesión persistente por stdin/stdout**: más rápida, pero obliga a gestionar reconexiones y el ciclo de vida de la sesión. Los comandos one-shot se pueden depurar a mano.
- **qBittorrent desde el PC**: exigiría exponer la WebUI en la red o abrir un túnel, y tener las credenciales en dos máquinas.

## Consequences

- El Agent tiene que estar instalado en el Remote y con la misma versión de protocolo. El servidor no usa Nix, así que hay que distribuir el binario a mano.
- Los Transfers usan `rsync` externo sobre el mismo alias SSH, así que el PC necesita `rsync` y `ssh`.
