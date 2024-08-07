/* https://developer.mozilla.org/en-US/docs/Web/Media/Formats/Image_types */
const supportedImages = [
    'image/apng',
    'image/avif',
    'image/gif',
    'image/jpeg',
    'image/png',
    'image/webp',
    'image/bmp',
    'image/tiff',
];

const files = { };
onmessage = async (event) => {
    const data = event.data;

    if (data.type === 'file') {
        const file = data.data;

        // TODO: Some formats supported by our backend aren't supported in browsers
        // e.g. FF just gives up on matroshka
        if (!file.type.startsWith('video/') && !supportedImages.includes(file.type)) {
            console.log('file', file,
                'upload cancelled because it is of an unsupported type');
            return;
        }
    
        const url = URL.createObjectURL(file);
        const format = file.type.split('/')[0];
        postMessage({ type: 'candidate', data: [format, url] });
    
        files[url] = file;
    } else if (data.type === 'begin-upload') {
        for (let [url, value] of Object.entries(files)) {
            const fd = new FormData();
            fd.append('file', value);

            const xhr = new XMLHttpRequest();
            xhr.open('POST', '/api/posts/upload', true);
            xhr.send(fd);

            delete files[url];
            postMessage({ type: 'uploaded', data: url });
        }
    } else if (data.type === 'remove-candidate') {
        const url = data.data;

        delete files[url];
    }
};