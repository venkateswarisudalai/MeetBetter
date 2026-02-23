// Simple script to create placeholder extension icons
// Run with: node create-icons.js

const fs = require('fs');
const path = require('path');

// Create SVG icons that can be converted to PNG
const sizes = [16, 48, 128];

sizes.forEach(size => {
  const svg = `
<svg width="${size}" height="${size}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#667eea;stop-opacity:1" />
      <stop offset="100%" style="stop-color:#764ba2;stop-opacity:1" />
    </linearGradient>
  </defs>
  <rect width="${size}" height="${size}" fill="url(#grad)" rx="${size * 0.15}"/>
  <text x="50%" y="55%" font-family="Arial, sans-serif" font-size="${size * 0.45}" font-weight="bold" fill="white" text-anchor="middle" dominant-baseline="middle">MB</text>
</svg>
  `.trim();

  const filepath = path.join(__dirname, 'icons', `icon${size}.svg`);
  fs.writeFileSync(filepath, svg);
  console.log(`Created ${filepath}`);
});

console.log('\nSVG icons created!');
console.log('\nTo convert to PNG (requires imagemagick):');
console.log('cd browser-extension/icons');
sizes.forEach(size => {
  console.log(`convert icon${size}.svg icon${size}.png`);
});

console.log('\nOr use an online converter: https://cloudconvert.com/svg-to-png');
console.log('\nFor now, you can also rename the SVG files to PNG:');
sizes.forEach(size => {
  console.log(`cp icon${size}.svg icon${size}.png`);
});
