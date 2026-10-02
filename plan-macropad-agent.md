# MacroPad Agent — plan de desarrollo

## Nombre propuesto

**MacroPad Agent**

Es un nombre claro para una aplicación personal que trabaja en segundo plano y administra los perfiles de un macro pad USB. El identificador técnico del proyecto puede ser `macropad-agent`.

## Objetivo

Crear una aplicación local para Linux y macOS que detecte el macro pad USB `1189:8890` al conectarlo y programe automáticamente el perfil adecuado para el sistema operativo.

La primera versión no usará cuentas, internet, servidores, telemetría ni tienda de aplicaciones.

## Experiencia esperada

```text
Conectas el macro pad
        ↓
El agente local detecta el USB 1189:8890
        ↓
Identifica Linux o macOS
        ↓
Selecciona el perfil correspondiente
        ↓
Programa el teclado y muestra una notificación breve
```

La aplicación tendrá dos componentes:

- Un agente que inicia automáticamente al abrir sesión y trabaja en segundo plano.
- Una ventana de configuración para editar perfiles, consultar el estado y aplicar un perfil manualmente.

## Perfil inicial: desarrollo de software

Se conservará la misma posición física para cada intención. Solo cambia el modificador según el sistema.

| Control | Linux | macOS |
|---|---|---|
| Tecla 1 | Guardar: `Ctrl+S` | Guardar: `Cmd+S` |
| Tecla 2 | Deshacer: `Ctrl+Z` | Deshacer: `Cmd+Z` |
| Tecla 3 | Rehacer: `Ctrl+Shift+Z` | Rehacer: `Cmd+Shift+Z` |
| Tecla 4 | Abrir archivo: `Ctrl+P` | Abrir archivo: `Cmd+P` |
| Tecla 5 | Paleta de comandos: `Ctrl+Shift+P` | Paleta de comandos: `Cmd+Shift+P` |
| Tecla 6 | Ejecutar o depurar: `F5` | Ejecutar o depurar: `F5` |
| Perilla | Volumen; pulsar para silenciar | Volumen; pulsar para silenciar |

Este perfil funciona especialmente bien con VS Code y editores que siguen atajos similares.

## Tecnología

Usar **Rust** para el núcleo y el agente. La herramienta compatible con este teclado, `ch57x-keyboard-tool`, ya está hecha en Rust y soporta el dispositivo `1189:8890`.

Usar **Tauri** para la ventana de configuración. Es una aplicación de escritorio ligera; el agente seguirá funcionando aunque la ventana esté cerrada.

## Estructura inicial del proyecto

```text
macropad-agent/
├── crates/
│   ├── core/              # perfiles, detección y programación USB
│   ├── daemon/            # proceso en segundo plano
│   └── desktop/           # aplicación de configuración Tauri
├── profiles/
│   ├── desarrollo-linux.yaml
│   └── desarrollo-macos.yaml
├── installers/
│   ├── linux/
│   └── macos/
└── docs/
    └── instalacion-local.md
```

## Formato de perfil inicial

```yaml
name: Desarrollo Linux
platform: linux
bindings:
  key_1: ctrl-s
  key_2: ctrl-z
  key_3: ctrl-shift-z
  key_4: ctrl-p
  key_5: ctrl-shift-p
  key_6: f5
  knob_left: volume_down
  knob_right: volume_up
  knob_press: mute
```

## Fases de desarrollo

### 1. Diagnóstico del hardware

Crear una utilidad pequeña que detecte el USB `1189:8890`, confirme las teclas y perilla disponibles, y aplique un perfil mínimo de prueba.

También debe comprobar si el teclado permite leer el perfil guardado o solamente escribirlo. Si solo permite escribir, el agente aplicará el perfil al detectar una conexión nueva.

### 2. Motor de perfiles

Crear un módulo que lea, valide y aplique perfiles YAML. Debe convertir acciones como `guardar` o `paleta de comandos` en las combinaciones específicas de Linux o macOS.

Guardar localmente el último perfil aplicado y el resultado de cada intento para diagnóstico.

### 3. Agente de segundo plano

El agente escuchará los eventos USB. Al aparecer el dispositivo:

1. Espera a que el sistema termine de crear sus interfaces USB.
2. Elige el perfil del sistema operativo.
3. Valida el perfil.
4. Programa el teclado.
5. Guarda el resultado y muestra una notificación breve.

También tendrá comandos manuales:

```bash
macropad-agent status
macropad-agent apply desarrollo-linux
macropad-agent diagnose
```

### 4. Inicio automático y permisos

En Linux se instalará una regla `udev` limitada al dispositivo `1189:8890`, junto con un servicio de usuario `systemd` que inicie al entrar a sesión.

En macOS se instalará un `LaunchAgent` para iniciar con la cuenta del usuario. La aplicación solicitará los permisos que macOS requiera para acceder al USB durante la primera ejecución.

### 5. Interfaz de configuración

La primera interfaz incluirá:

- Estado de conexión y perfil aplicado.
- Selector de perfil.
- Representación de seis teclas y una perilla.
- Lista de acciones comunes: teclas, combinaciones, multimedia y desplazamiento.
- Botón para aplicar el perfil ahora.
- Botón para restaurar el perfil de desarrollo.
- Vista de diagnóstico para ver o copiar errores.

### 6. Instalación local

Se generarán instaladores solo para tus equipos:

| Sistema | Entregable |
|---|---|
| Linux | Paquete `.deb` |
| macOS | Aplicación `.app` y archivo `.dmg` local |

Los perfiles creados por el usuario se guardarán fuera de la aplicación para que sobrevivan a las actualizaciones:

```text
Linux: ~/.config/macropad-agent/
macOS: ~/Library/Application Support/MacroPad Agent/
```

## Orden recomendado para empezar

1. Crear el repositorio local `macropad-agent`.
2. Crear el proyecto Rust y un comando de diagnóstico USB.
3. Aplicar manualmente un perfil de prueba desde Linux.
4. Crear y validar los perfiles de desarrollo para Linux y macOS.
5. Desarrollar el agente que detecta el USB y aplica el perfil en Linux.
6. Instalar el servicio y probar reconexiones del teclado.
7. Adaptar el agente a macOS.
8. Construir la interfaz de configuración.
9. Empaquetar los instaladores locales.

## Alcance posterior

Después de que el cambio automático por sistema operativo sea confiable, se puede añadir cambio de perfil por aplicación activa, por ejemplo: VS Code, terminal, reuniones o edición multimedia.
