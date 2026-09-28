import { NodeConfig, SlskrNode } from './SlskrNode';

/**
 * Manages multiple slskr test nodes for multi-peer scenarios.
 * Handles lifecycle (start/stop) and provides access to nodes.
 */
export class MultiPeerHarness {
  private nodes: Map<string, SlskrNode> = new Map();
  private peerPorts: Map<string, number> = new Map();

  /**
   * Start a new test node.
   * @param name Node name (A, B, C, etc.)
   * @param shareDir Single share directory path, or array of paths for multiple shares
   * @param flags Optional flags (noConnect, etc.)
   */
  async startNode(
    name: string,
    shareDir: string | string[],
    flags?: NonNullable<NodeConfig['flags']>,
  ): Promise<SlskrNode> {
    if (this.nodes.has(name)) {
      throw new Error(`Node ${name} already exists`);
    }

    if (this.peerPorts.size === 0) {
      const names = new Set(['A', 'B', 'C', name]);
      const ports = await Promise.all(
        [...names].map(async (peerName) => [
          peerName,
          await SlskrNode.allocateFreePort(),
        ] as const),
      );
      this.peerPorts = new Map(ports);
    } else if (!this.peerPorts.has(name)) {
      this.peerPorts.set(name, await SlskrNode.allocateFreePort());
    }

    // Small delay between starting nodes to avoid lock file conflicts
    if (this.nodes.size > 0) {
      await new Promise((resolve) => setTimeout(resolve, 1_000));
    }

    const endpointOverrides = Object.fromEntries(
      [...this.peerPorts].map(([peerName, port]) => [
        `node${peerName}`,
        `127.0.0.1:${port}`,
      ]),
    );
    const node = new SlskrNode({
      flags: {
        ...flags,
        peerPort: this.peerPorts.get(name),
        reservedPeerPorts: [...this.peerPorts.values()],
        endpointOverrides: {
          ...endpointOverrides,
          ...flags?.endpointOverrides,
        },
      },
      nodeName: name,
      shareDir,
    });

    this.nodes.set(name, node);
    try {
      await node.start();
      return node;
    } catch (error) {
      await node.stop();
      this.nodes.delete(name);
      throw error;
    }
  }

  /**
   * Get a node by name.
   */
  getNode(name: string): SlskrNode {
    const node = this.nodes.get(name);
    if (!node) {
      throw new Error(`Node ${name} not found`);
    }

    return node;
  }

  /**
   * Stop all nodes and clean up.
   */
  async stopAll(): Promise<void> {
    await Promise.all([...this.nodes.values()].map((node) => node.stop()));
    this.nodes.clear();
  }

  /**
   * Get all node names.
   */
  getNodeNames(): string[] {
    return [...this.nodes.keys()];
  }
}
