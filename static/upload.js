const selectFiles = document.getElementById('select-files');
const candidiates = document.getElementById('candidates');
const beginUpload = document.getElementById('begin-upload');
const imageCandidiateTemplate = document.getElementById('image-candidate-template');
const videoCandidiateTemplate = document.getElementById('video-candidiate-template');

const worker = new Worker('/static/upload_worker.js');
worker.onmessage = (event) => {
    const [format, url] = event.data;
    
    let candidate;
    if (format === 'image') {
        candidate = imageCandidiateTemplate.cloneNode(true).content;
        candidate.querySelector('img').src = url;
    } else if (format === 'video') {
        candidate = videoCandidiateTemplate.cloneNode(true).content;
        candidate.querySelector('video').src = url;
    }

    candidate.querySelector('.edit-button').addEventListener('click', () => {
        // TODO
    });
    candidate.querySelector('.trash-button').addEventListener('click', (e) => {
        e.target.closest('.candidate').remove();
    });

    candidiates.appendChild(candidate);
};

beginUpload.addEventListener('click', () => {
    worker.postMessage({ type: 'begin-upload', });
});

selectFiles.addEventListener('click', () => {
    const input = document.createElement('input');
    input.setAttribute('type', 'file');
    input.setAttribute('multiple', '');
    input.addEventListener('change', async (evt) => {
        for (const file of evt.originalTarget.files) {
            worker.postMessage({ type: 'file', data: file, });
        }
    });

    input.click();
});

document.body.addEventListener('dragover', (e) => {
    e.preventDefault();
});

document.body.addEventListener('dragleave', (e) => {
    e.preventDefault();
});

document.body.addEventListener('drop', (e) => {
    e.preventDefault();

    Array.from(e.dataTransfer.files).forEach(x => worker.postMessage({ type: 'file', data: x, }));
});