# Ratatidy

Gestión de una biblioteca multimedia y de sus descargas de torrent, cuidando que los hardlinks entre ambas sigan intactos.

## Biblioteca

**Download**:
Contenido que qBittorrent ha descargado y del que hace seed, dentro del directorio de descargas.
_Avoid_: torrent (para referirse a los ficheros)

**Media**:
Contenido organizado por Radarr/Sonarr dentro de los directorios de medios.
_Avoid_: library folder

**Group**:
Una película, una serie o un Download, visto como una unidad sobre la que se actúa.
_Avoid_: item, entry

**Container**:
La carpeta (o el fichero) que representa físicamente un Group y sobre la que actúan las acciones.

**Orphan**:
Un fichero que existe solo en Downloads o solo en Media, sin hardlink en el otro lado.

## Máquinas

**Host**:
La máquina donde viven Downloads y Media y donde se escanea, se borra y se habla con qBittorrent. Puede ser la misma máquina en la que corre la TUI (local) o otra (remota).
_Avoid_: server (como término del dominio)

**Remote**:
Un Host al que la TUI accede por SSH desde otra máquina.
_Avoid_: destino (Remote es siempre el origen de los datos)

**Agent**:
Ratatidy ejecutándose en un Remote para atender, por SSH, las peticiones de la TUI.
_Avoid_: daemon, server

**Transfer**:
Copia del Container de un Group desde un Remote a la máquina en la que corre la TUI. El origen nunca se modifica.
_Avoid_: download, sync, fetch
