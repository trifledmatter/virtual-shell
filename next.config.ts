import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
  webpack: {
    // @ts-ignore
    experiments: {
      asyncWebAssembly: true
    }
  }
};

export default nextConfig;
