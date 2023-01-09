// @ts-check

/**
 * @type {import('next').NextConfig}
 **/

const withPWA = require('next-pwa')({
  dest: 'public',
  register: true,
  publicExcludes: ['!runtime-config.json'],
})

// @ts-ignore
module.exports = withPWA({
  // next.js config
  reactStrictMode: true,
})
