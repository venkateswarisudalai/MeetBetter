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
  window.addEventListener('scroll', () => {
    if (window.pageYOffset > 60) {
      navbar.style.borderBottomColor = '#e5e5e3';
    } else {
      navbar.style.borderBottomColor = 'transparent';
    }
  });
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

// ===== Subtle Fade-in Animations =====
function setupFadeAnimations() {
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('fade-in');
        observer.unobserve(entry.target);
      }
    });
  }, {
    threshold: 0.1,
    rootMargin: '0px 0px -60px 0px'
  });

  document.querySelectorAll('.feature-row, .step, .download-content').forEach(el => {
    el.style.opacity = '0';
    el.style.transform = 'translateY(20px)';
    el.style.transition = 'opacity 0.5s ease, transform 0.5s ease';
    observer.observe(el);
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
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
