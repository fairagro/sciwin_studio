**SciWIn-Studio** is a graphical user interface (GUI) application currently in testing that complements SciWIn-Client. It provides an intuitive visual environment for researchers who prefer graphical tools over command-line interactions.
### Features
- Visual workflow design and management
- Drag-and-drop interface for connecting workflow steps
- Real-time workflow visualization
- Accessible workflow creation without terminal expertise

<img src=".github/studio.png" alt="Screenshot of SciWIn Studio" width=750>

### Running SciWIn-Studio
Builds of SciWIn-Studio can be found in the [Actions-Tab](https://github.com/fairagro/sciwin/actions/workflows/bundle.yml) until it is released properly.
To run SciWIn-Studio in **Development mode**, you need to [install the Dioxus CLI `dx`](https://dioxuslabs.com/learn/0.7/getting_started/):
```bash
# Install requirements
sudo apt-get update 
sudo apt-get install -y \
    libgtk-3-dev \
    libglib2.0-dev \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libxdo-dev \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# Install Dioxus CLI
curl -sSL https://dioxus.dev/install.sh | bash

# or (slower)
cargo install dioxus-cli

# Navigate to the project directory
cd sciwin

# Launch SciWIn-Studio in debug mode
dx serve -p sciwin
```
> [!NOTE]
> SciWIn-Studio is currently in testing phase. Features and functionality may change as development progresses.

# Development

Your new bare-bones project includes minimal organization with a single `main.rs` file and a few assets.

```
project/
├─ assets/ # Any assets that are used by the app should be placed here
├─ src/
│  ├─ main.rs # main.rs is the entry point to your application and currently contains all components for the app
├─ Cargo.toml # The Cargo.toml file defines the dependencies and feature flags for your project
```

### Tailwind
1. Install npm: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
2. Install the Tailwind CSS CLI: https://tailwindcss.com/docs/installation
3. Run the following command in the root of the project to start the Tailwind CSS compiler:

```bash
npx tailwindcss -i ./tailwind.css -o ./assets/tailwind.css --watch
```

### Serving Your App

Run the following command in the root of your project to start developing with the default platform:

```bash
dx serve
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```

