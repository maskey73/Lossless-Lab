import { create } from "zustand";
import type { View, ReplayGainMode, AudioDeviceInfo, DeviceProfile } from "../lib/types";
import * as cmd from "../lib/tauri-commands";

interface SettingsState {
  // Navigation
  currentView: View;
  setView: (view: View) => void;

  // Audio devices
  devices: AudioDeviceInfo[];
  selectedDevice: string | null;
  loadDevices: () => Promise<void>;

  // ReplayGain
  replaygainMode: ReplayGainMode;
  clippingPrevention: boolean;
  setReplaygainMode: (mode: ReplayGainMode) => Promise<void>;
  setClippingPrevention: (enabled: boolean) => Promise<void>;

  // Device profiles
  profiles: DeviceProfile[];
  loadProfiles: () => Promise<void>;
  saveProfile: (profile: DeviceProfile) => Promise<void>;
  deleteProfile: (deviceName: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  currentView: "now-playing",
  setView: (view) => set({ currentView: view }),

  devices: [],
  selectedDevice: null,
  loadDevices: async () => {
    try {
      const devices = await cmd.getAudioDevices();
      const defaultDev = devices.find((d) => d.is_default);
      set({
        devices,
        selectedDevice: defaultDev?.name ?? devices[0]?.name ?? null,
      });
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  },

  replaygainMode: "Off",
  clippingPrevention: true,

  setReplaygainMode: async (mode) => {
    try {
      await cmd.setReplaygainMode(mode);
      set({ replaygainMode: mode });
    } catch (e) {
      console.error("Failed to set ReplayGain mode:", e);
    }
  },

  setClippingPrevention: async (enabled) => {
    try {
      await cmd.setClippingPrevention(enabled);
      set({ clippingPrevention: enabled });
    } catch (e) {
      console.error("Failed to set clipping prevention:", e);
    }
  },

  profiles: [],
  loadProfiles: async () => {
    try {
      const profiles = await cmd.listDeviceProfiles();
      set({ profiles });
    } catch (e) {
      console.error("Failed to load profiles:", e);
    }
  },

  saveProfile: async (profile) => {
    try {
      await cmd.saveDeviceProfile(profile);
      const profiles = await cmd.listDeviceProfiles();
      set({ profiles });
    } catch (e) {
      console.error("Failed to save profile:", e);
    }
  },

  deleteProfile: async (deviceName) => {
    try {
      await cmd.deleteDeviceProfile(deviceName);
      const profiles = await cmd.listDeviceProfiles();
      set({ profiles });
    } catch (e) {
      console.error("Failed to delete profile:", e);
    }
  },
}));
