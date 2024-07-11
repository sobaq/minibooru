const selectFiles = document.getElementById('select-files');
const candidiates = document.getElementById('candidates');
const beginUpload = document.getElementById('begin-upload');
const imageCandidiateTemplate = document.getElementById('image-candidate-template');
const videoCandidiateTemplate = document.getElementById('video-candidiate-template');

const worker = new Worker('/static/upload_worker.js');
worker.onmessage = (event) => {
    const data = event.data;

    if (data.type === 'candidate') {
        const [format, url] = data.data;
    
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
            const candidate = e.target.closest('.candidate');
            
            worker.postMessage({ type: 'remove-candidate', data: candidate.src, });
            candidate.remove();
        });
    
        candidiates.appendChild(candidate);
    } else if (data.type === 'uploaded') {
        const url = data.data;
        const candidate = candidiates.querySelector(`[src="${CSS.escape(url)}"]`);

        candidate.remove();
    }
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
            worker.postMessage({ type: 'file', data: file });
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