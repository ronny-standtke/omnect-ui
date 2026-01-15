import { Page } from '@playwright/test';
import { publishToCentrifugo } from './centrifugo';

export interface DeviceNetwork {
  name: string;
  mac: string;
  online: boolean;
  ipv4?: {
    addrs: Array<{ addr: string; dhcp: boolean; prefix_len: number }>;
    dns: string[];
    gateways: string[];
  };
}

export interface NetworkTestHarnessConfig {
  rollbackTimeoutSeconds?: number;
  enableHealthcheckPolling?: boolean;
  healthcheckSuccessAfter?: number; // ms
  healthcheckAlwaysFails?: boolean;
}

export interface SetNetworkConfigResponse {
  rollbackTimeoutSeconds: number;
  uiPort: number;
  rollbackEnabled: boolean;
}

export class NetworkTestHarness {
  private rollbackEnabled: boolean = false;
  private rollbackDeadline: number | null = null;
  private currentIp: string = '192.168.1.100';
  private newIp: string | null = null;
  private healthcheckCallCount: number = 0;
  private healthcheckConfig: NetworkTestHarnessConfig = {};
  private networkRollbackOccurred: boolean = false;
  private lastNetworkConfig: DeviceNetwork[] = [];

  /**
   * Mock the /network endpoint with configurable response
   */
  async mockNetworkConfig(page: Page, config: NetworkTestHarnessConfig = {}): Promise<void> {
    await page.route('**/network', async (route) => {
      if (route.request().method() === 'POST') {
        const requestBody = route.request().postDataJSON();

        // Track rollback state
        this.rollbackEnabled = requestBody.enableRollback === true;

        // If rollback enabled, set deadline
        if (this.rollbackEnabled) {
          const timeoutSeconds = config.rollbackTimeoutSeconds || 90;
          this.rollbackDeadline = Date.now() + (timeoutSeconds * 1000);
        }

        // Track IP change
        if (requestBody.ip && requestBody.ip !== this.currentIp) {
          this.newIp = requestBody.ip;
        }

        // Reset healthcheck counter
        this.healthcheckCallCount = 0;
        this.healthcheckConfig = config;

        // Send success response
        const response: SetNetworkConfigResponse = {
          rollbackTimeoutSeconds: config.rollbackTimeoutSeconds || 90,
          uiPort: 5173,
          rollbackEnabled: this.rollbackEnabled,
        };

        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(response),
        });
      } else {
        await route.continue();
      }
    });
  }

  /**
   * Mock the /network endpoint to return an error
   */
  async mockNetworkConfigError(page: Page, statusCode: number = 500, errorMessage: string = 'Internal server error'): Promise<void> {
    await page.route('**/network', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: statusCode,
          contentType: 'application/json',
          body: JSON.stringify({ error: errorMessage }),
        });
      } else {
        await route.continue();
      }
    });
  }

  /**
   * Mock the /healthcheck endpoint with configurable responses
   */
  async mockHealthcheck(page: Page, config: NetworkTestHarnessConfig = {}): Promise<void> {
    this.healthcheckConfig = config;

    await page.route('**/healthcheck', async (route) => {
      this.healthcheckCallCount++;

      // Determine if healthcheck should succeed
      let healthcheckSucceeds = false;

      if (config.healthcheckAlwaysFails) {
        healthcheckSucceeds = false;
      } else if (config.healthcheckSuccessAfter !== undefined) {
        // Calculate elapsed time since first healthcheck
        const elapsedMs = (this.healthcheckCallCount - 1) * 2000; // Assuming 2s poll interval
        healthcheckSucceeds = elapsedMs >= config.healthcheckSuccessAfter;
      } else {
        // Default: succeed after a few attempts
        healthcheckSucceeds = this.healthcheckCallCount >= 3;
      }

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          version_info: {
            required: '>=0.39.0',
            current: '0.40.0',
            mismatch: false,
          },
          update_validation_status: {
            status: 'valid',
          },
          network_rollback_occurred: this.networkRollbackOccurred,
        }),
      });
    });
  }

  /**
   * Mock /ack-rollback endpoint
   */
  async mockAckRollback(page: Page): Promise<void> {
    await page.route('**/ack-rollback', async (route) => {
      if (route.request().method() === 'POST') {
        this.networkRollbackOccurred = false;
        await route.fulfill({
          status: 200,
        });
      }
    });
  }

  /**
   * Publish network status via Centrifugo
   */
  async publishNetworkStatus(adapters: DeviceNetwork[]): Promise<void> {
    this.lastNetworkConfig = adapters;
    await publishToCentrifugo('NetworkStatusV1', {
      network_status: adapters,
    });
  }

  /**
   * Simulate automatic rollback after timeout
   * This sets the rollback occurred flag and reverts network status
   */
  async simulateRollbackTimeout(): Promise<void> {
    if (!this.rollbackEnabled) {
      throw new Error('Cannot simulate rollback timeout: rollback was not enabled');
    }

    this.networkRollbackOccurred = true;
    this.rollbackEnabled = false;
    this.rollbackDeadline = null;

    // Revert IP to original
    if (this.newIp) {
      this.newIp = null;
    }

    // Publish reverted network status
    const revertedAdapters = this.lastNetworkConfig.map(adapter => ({
      ...adapter,
      ipv4: adapter.ipv4 ? {
        ...adapter.ipv4,
        addrs: adapter.ipv4.addrs.map(addr => ({
          ...addr,
          addr: this.currentIp,
        })),
      } : undefined,
    }));

    await this.publishNetworkStatus(revertedAdapters);
  }

  /**
   * Simulate successful connection to new IP (cancels rollback)
   */
  async simulateNewIpReachable(): Promise<void> {
    if (!this.rollbackEnabled) {
      throw new Error('Cannot simulate new IP reachable: rollback was not enabled');
    }

    // Cancel rollback
    this.rollbackEnabled = false;
    this.rollbackDeadline = null;

    // Update current IP to new IP
    if (this.newIp) {
      this.currentIp = this.newIp;
      this.newIp = null;
    }
  }

  /**
   * Create a standard network adapter with customizable config
   */
  createAdapter(name: string, config: Partial<DeviceNetwork> = {}): DeviceNetwork {
    return {
      name,
      mac: config.mac || '00:11:22:33:44:55',
      online: config.online !== undefined ? config.online : true,
      ipv4: config.ipv4 || {
        addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }],
        dns: ['8.8.8.8'],
        gateways: ['192.168.1.1'],
      },
    };
  }

  /**
   * Create multiple adapters for testing multi-adapter scenarios
   */
  createMultipleAdapters(count: number): DeviceNetwork[] {
    const names = ['eth0', 'eth1', 'wlan0', 'eth2', 'wlan1'];
    const macs = [
      '00:11:22:33:44:55',
      '00:11:22:33:44:56',
      '00:11:22:33:44:57',
      '00:11:22:33:44:58',
      '00:11:22:33:44:59',
    ];

    return Array.from({ length: Math.min(count, 5) }, (_, i) => ({
      name: names[i],
      mac: macs[i],
      online: true,
      ipv4: {
        addrs: [{ addr: `192.168.1.${100 + i}`, dhcp: false, prefix_len: 24 }],
        dns: ['8.8.8.8'],
        gateways: ['192.168.1.1'],
      },
    }));
  }

  /**
   * Set the rollback occurred flag (for testing rollback notification)
   */
  setRollbackOccurred(occurred: boolean): void {
    this.networkRollbackOccurred = occurred;
  }

  /**
   * Get current rollback state
   */
  getRollbackState(): { enabled: boolean; deadline: number | null; occurred: boolean } {
    return {
      enabled: this.rollbackEnabled,
      deadline: this.rollbackDeadline,
      occurred: this.networkRollbackOccurred,
    };
  }

  /**
   * Get healthcheck call count
   */
  getHealthcheckCallCount(): number {
    return this.healthcheckCallCount;
  }

  /**
   * Reset harness state (for test cleanup)
   */
  reset(): void {
    this.rollbackEnabled = false;
    this.rollbackDeadline = null;
    this.currentIp = '192.168.1.100';
    this.newIp = null;
    this.healthcheckCallCount = 0;
    this.healthcheckConfig = {};
    this.networkRollbackOccurred = false;
    this.lastNetworkConfig = [];
  }
}
