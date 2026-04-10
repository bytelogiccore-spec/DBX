// DBX Native Node.js bindings
const fs = require('fs');
const path = require('path');

const isNode = f => f.endsWith('.node');
const nodes = fs.readdirSync(__dirname).filter(isNode);

if (nodes.length === 1) {
    module.exports = require(path.join(__dirname, nodes[0]));
} else if (nodes.includes('dbx-native.node')) {
    module.exports = require(path.join(__dirname, 'dbx-native.node'));
} else if (nodes.includes('index.node')) {
    module.exports = require(path.join(__dirname, 'index.node'));
} else {
    const { platform, arch } = process;
    const load = (name) => {
        try { return require(path.join(__dirname, name)); } catch (e) { return null; }
    };
    const mod = load(`dbx-native.${platform}-${arch}-gnu.node`) ||
                load(`dbx-native.${platform}-${arch}-msvc.node`) ||
                load(`dbx-native.${platform}-${arch}.node`);
    if (!mod) throw new Error(`Could not find dbx-native binding for ${platform}-${arch}`);
    module.exports = mod;
}
