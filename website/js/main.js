// MeetBetter Landing Page - Main JavaScript

// ===== OS Detection =====
function detectOS() {
  const platform = window.navigator.platform.toLowerCase();
  const userAgent = window.navigator.userAgent.toLowerCase();

  if (platform.includes('mac') || userAgent.includes('mac')) {
    return 'mac';
  } else if (platform.includes('win') || userAgent.includes('win')) {
    return 'windows';
  } else if (platform.includes('linux') || userAgent.includes('linux') || userAgent.includes('x11')) {
    return 'linux';
  }

  return 'unknown';
}

// Update UI based on detected OS
function highlightOS() {
  const os = detectOS();
  const osNames = {
    mac: 'Mac',
    windows: 'Windows',
    linux: 'Linux',
    unknown: 'Your OS'
  };

  // Update primary download button text
  const osNameEl = document.getElementById('os-name');
  if (osNameEl) {
    osNameEl.textContent = osNames[os];
  }

  // Highlight the corresponding download card
  const downloadCards = {
    mac: document.getElementById('download-mac'),
    windows: document.getElementById('download-windows'),
    linux: document.getElementById('download-linux')
  };

  if (downloadCards[os]) {
    downloadCards[os].classList.add('highlighted');
    // Scroll into view on mobile
    if (window.innerWidth < 768) {
      setTimeout(() => {
        downloadCards[os].scrollIntoView({
          behavior: 'smooth',
          block: 'nearest'
        });
      }, 500);
    }
  }
}

// ===== Smooth Scrolling =====
function setupSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
      e.preventDefault();
      const targetId = this.getAttribute('href');

      if (targetId === '#') return;

      const targetElement = document.querySelector(targetId);
      if (targetElement) {
        const navbarHeight = document.querySelector('.navbar').offsetHeight;
        const elementPosition = targetElement.getBoundingClientRect().top;
        const offsetPosition = elementPosition + window.pageYOffset - navbarHeight - 20;

        window.scrollTo({
          top: offsetPosition,
          behavior: 'smooth'
        });

        // Close mobile menu if open
        closeMobileMenu();
      }
    });
  });
}

// ===== Navbar Scroll Effect =====
function setupNavbarScroll() {
  const navbar = document.querySelector('.navbar');
  let lastScroll = 0;

  window.addEventListener('scroll', () => {
    const currentScroll = window.pageYOffset;

    if (currentScroll > 100) {
      navbar.classList.add('scrolled');
    } else {
      navbar.classList.remove('scrolled');
    }

    lastScroll = currentScroll;
  });
}

// ===== Mobile Menu =====
function setupMobileMenu() {
  const mobileMenuToggle = document.querySelector('.mobile-menu-toggle');
  const navLinks = document.querySelector('.nav-links');

  if (!mobileMenuToggle) return;

  mobileMenuToggle.addEventListener('click', () => {
    navLinks.classList.toggle('active');
    mobileMenuToggle.classList.toggle('active');

    // Animate hamburger icon
    const spans = mobileMenuToggle.querySelectorAll('span');
    if (mobileMenuToggle.classList.contains('active')) {
      spans[0].style.transform = 'rotate(45deg) translate(5px, 5px)';
      spans[1].style.opacity = '0';
      spans[2].style.transform = 'rotate(-45deg) translate(7px, -6px)';
    } else {
      spans[0].style.transform = 'none';
      spans[1].style.opacity = '1';
      spans[2].style.transform = 'none';
    }
  });
}

function closeMobileMenu() {
  const navLinks = document.querySelector('.nav-links');
  const mobileMenuToggle = document.querySelector('.mobile-menu-toggle');

  if (navLinks && navLinks.classList.contains('active')) {
    navLinks.classList.remove('active');
    mobileMenuToggle.classList.remove('active');

    const spans = mobileMenuToggle.querySelectorAll('span');
    spans[0].style.transform = 'none';
    spans[1].style.opacity = '1';
    spans[2].style.transform = 'none';
  }
}

// ===== Scroll Animations =====
function setupScrollAnimations() {
  const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -100px 0px'
  };

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('animate-in');
        observer.unobserve(entry.target);
      }
    });
  }, observerOptions);

  // Observe all feature cards, steps, and download cards
  document.querySelectorAll('.feature-card, .step, .download-card').forEach(el => {
    el.style.opacity = '0';
    el.style.transform = 'translateY(30px)';
    el.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
    observer.observe(el);
  });

  // Add animate-in class styles
  const style = document.createElement('style');
  style.textContent = `
    .animate-in {
      opacity: 1 !important;
      transform: translateY(0) !important;
    }
  `;
  document.head.appendChild(style);
}

// ===== Download Tracking (Optional Analytics) =====
function setupDownloadTracking() {
  document.querySelectorAll('.btn-download, .btn-outline').forEach(button => {
    button.addEventListener('click', (e) => {
      const platform = button.closest('.download-card')?.querySelector('.download-title')?.textContent || 'Extension';

      // You can send this to your analytics service
      console.log(`Download clicked: ${platform}`);

      // Example: Google Analytics
      // gtag('event', 'download', { 'platform': platform });
    });
  });
}

// ===== Copy to Clipboard Helper =====
function copyToClipboard(text) {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(text).then(() => {
      showNotification('Copied to clipboard!');
    });
  } else {
    // Fallback for older browsers
    const textarea = document.createElement('textarea');
    textarea.value = text;
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand('copy');
    document.body.removeChild(textarea);
    showNotification('Copied to clipboard!');
  }
}

// ===== Notification Toast =====
function showNotification(message) {
  const notification = document.createElement('div');
  notification.className = 'notification-toast';
  notification.textContent = message;

  // Add toast styles
  Object.assign(notification.style, {
    position: 'fixed',
    bottom: '24px',
    right: '24px',
    padding: '16px 24px',
    background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
    color: 'white',
    borderRadius: '12px',
    boxShadow: '0 8px 16px rgba(102, 126, 234, 0.3)',
    zIndex: '10000',
    fontSize: '14px',
    fontWeight: '600',
    animation: 'slideInUp 0.3s ease'
  });

  // Add animation
  const style = document.createElement('style');
  style.textContent = `
    @keyframes slideInUp {
      from {
        transform: translateY(100px);
        opacity: 0;
      }
      to {
        transform: translateY(0);
        opacity: 1;
      }
    }
  `;
  document.head.appendChild(style);

  document.body.appendChild(notification);

  // Remove after 3 seconds
  setTimeout(() => {
    notification.style.animation = 'slideInUp 0.3s ease reverse';
    setTimeout(() => notification.remove(), 300);
  }, 3000);
}

// ===== Enhanced Mobile Menu Styles =====
function addMobileMenuStyles() {
  const style = document.createElement('style');
  style.textContent = `
    @media (max-width: 768px) {
      .nav-links {
        position: fixed;
        top: 70px;
        left: 0;
        right: 0;
        background: white;
        flex-direction: column;
        padding: 24px;
        gap: 16px !important;
        box-shadow: 0 8px 16px rgba(0, 0, 0, 0.1);
        transform: translateY(-100%);
        opacity: 0;
        pointer-events: none;
        transition: all 0.3s ease;
      }

      .nav-links.active {
        transform: translateY(0);
        opacity: 1;
        pointer-events: all;
      }

      .nav-links a {
        font-size: 18px;
        padding: 12px 0;
      }
    }
  `;
  document.head.appendChild(style);
}

// ===== Lazy Loading Images (for future use) =====
function setupLazyLoading() {
  if ('IntersectionObserver' in window) {
    const imageObserver = new IntersectionObserver((entries, observer) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          const img = entry.target;
          img.src = img.dataset.src;
          img.classList.remove('lazy');
          imageObserver.unobserve(img);
        }
      });
    });

    document.querySelectorAll('img[data-src]').forEach(img => {
      imageObserver.observe(img);
    });
  }
}

// ===== Keyboard Navigation =====
function setupKeyboardNavigation() {
  document.addEventListener('keydown', (e) => {
    // ESC to close mobile menu
    if (e.key === 'Escape') {
      closeMobileMenu();
    }
  });
}

// ===== Initialize Everything =====
function init() {
  // Detect and highlight OS
  highlightOS();

  // Setup interactions
  setupSmoothScroll();
  setupNavbarScroll();
  setupMobileMenu();
  setupScrollAnimations();
  setupDownloadTracking();
  setupLazyLoading();
  setupKeyboardNavigation();

  // Add mobile menu styles
  addMobileMenuStyles();

  // Log successful initialization
  console.log('🎯 MeetBetter website initialized');
  console.log(`Detected OS: ${detectOS()}`);
}

// Run on DOM ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

// Expose functions for external use
window.MeetBetter = {
  copyToClipboard,
  showNotification,
  detectOS
};
