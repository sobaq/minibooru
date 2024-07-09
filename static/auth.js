const form = document.getElementById('form');
const error = document.getElementById('error');
const logOut = document.getElementById('log-out');

form.addEventListener('submit', async (e) => {
    e.preventDefault();

    const formData = new URLSearchParams(new FormData(form));
    const options = {
        method: 'POST',
        headers: new Headers({ 'Content-Type': 'application/x-www-form-urlencoded' }),
        body: formData,
    };

    let response;
    if (e.submitter.id === 'register') {
        response = await fetch('/api/auth/register', options);
    } else if (e.submitter.id === 'login') {
        response = await fetch('/api/auth/login', options);
    }

    if (!response.ok) {
        error.innerText = `${response.status} ${response.statusText}`
        const body = await response.text();
        if (body !== '') {
            error.innerText += `: ${body}`;
        }
    } else {
        window.location.href = '/';
    }
});