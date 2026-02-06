const rawBasePath = process.env.NEXT_PUBLIC_BASE_PATH;
const basePath =
  rawBasePath && rawBasePath !== '/'
    ? rawBasePath.replace(/\/$/, '')
    : undefined;

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  basePath,
  assetPrefix: basePath,
};

export default nextConfig;
