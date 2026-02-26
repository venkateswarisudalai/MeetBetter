// Vantage Landing Page

// ===== Smooth Scrolling =====
function setupSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
      e.preventDefault();
      const targetId = this.getAttribute('href');
      if (targetId === '#') return;

      const target = document.querySelector(targetId);
      if (target) {
        const navbarHeight = document.querySelector('.navbar').offsetHeight;
        const top = target.getBoundingClientRect().top + window.pageYOffset - navbarHeight - 20;
        window.scrollTo({ top, behavior: 'smooth' });
        closeMobileMenu();
      }
    });
  });
}

// ===== Navbar Scroll Effect =====
function setupNavbarScroll() {
  const navbar = document.querySelector('.navbar');
  const update = () => {
    if (window.pageYOffset > 40) {
      navbar.classList.add('scrolled');
    } else {
      navbar.classList.remove('scrolled');
    }
  };
  window.addEventListener('scroll', update, { passive: true });
  update();
}

// ===== Mobile Menu =====
function setupMobileMenu() {
  const toggle = document.querySelector('.mobile-menu-toggle');
  const navLinks = document.querySelector('.nav-links');
  if (!toggle) return;

  toggle.addEventListener('click', () => {
    navLinks.classList.toggle('active');
    toggle.classList.toggle('active');

    const spans = toggle.querySelectorAll('span');
    if (toggle.classList.contains('active')) {
      spans[0].style.transform = 'rotate(45deg) translate(5px, 5px)';
      spans[1].style.opacity = '0';
      spans[2].style.transform = 'rotate(-45deg) translate(5px, -5px)';
    } else {
      spans[0].style.transform = 'none';
      spans[1].style.opacity = '1';
      spans[2].style.transform = 'none';
    }
  });
}

function closeMobileMenu() {
  const navLinks = document.querySelector('.nav-links');
  const toggle = document.querySelector('.mobile-menu-toggle');
  if (navLinks && navLinks.classList.contains('active')) {
    navLinks.classList.remove('active');
    toggle.classList.remove('active');
    const spans = toggle.querySelectorAll('span');
    spans[0].style.transform = 'none';
    spans[1].style.opacity = '1';
    spans[2].style.transform = 'none';
  }
}

// ===== Fade-in Animations =====
function setupFadeAnimations() {
  const easing = 'cubic-bezier(0.22, 1, 0.36, 1)';

  // Staggered IntersectionObserver
  const observer = new IntersectionObserver((entries) => {
    const visible = entries
      .filter(e => e.isIntersecting)
      .sort((a, b) => a.target.dataset.fadeIndex - b.target.dataset.fadeIndex);

    visible.forEach((entry, i) => {
      const delay = i * 0.1;
      entry.target.style.transitionDelay = delay + 's';
      entry.target.classList.add('fade-in');
      observer.unobserve(entry.target);
    });
  }, {
    threshold: 0.08,
    rootMargin: '0px 0px -60px 0px'
  });

  // Hero elements — animate immediately with stagger
  const heroEls = document.querySelectorAll('.hero-badge, .hero-title, .hero-subtitle, .hero-ctas, .hero-meta');
  heroEls.forEach((el, i) => {
    el.style.opacity = '0';
    el.style.transform = 'translateY(20px)';
    el.style.transition = `opacity 0.7s ${easing}, transform 0.7s ${easing}`;
    el.style.transitionDelay = (0.1 + i * 0.1) + 's';
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        el.classList.add('fade-in');
      });
    });
  });

  // Scrollable elements
  let index = 0;
  document.querySelectorAll('.platforms, .feature-card, .step-card, .download-card').forEach(el => {
    el.dataset.fadeIndex = index++;
    el.style.opacity = '0';
    el.style.transform = 'translateY(28px)';
    el.style.transition = `opacity 0.7s ${easing}, transform 0.7s ${easing}`;
    observer.observe(el);
  });
}

// ===== Marquee hover pause =====
function setupMarquee() {
  const scroll = document.querySelector('.platforms-scroll');
  if (!scroll) return;

  scroll.addEventListener('mouseenter', () => {
    scroll.style.animationPlayState = 'paused';
  });
  scroll.addEventListener('mouseleave', () => {
    scroll.style.animationPlayState = 'running';
  });
}

// ===== Keyboard Navigation =====
function setupKeyboardNav() {
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeMobileMenu();
  });
}

// ===== Init =====
function init() {
  setupSmoothScroll();
  setupNavbarScroll();
  setupMobileMenu();
  setupFadeAnimations();
  setupKeyboardNav();
  setupMarquee();
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
