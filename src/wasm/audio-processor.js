// Audio worklet processor for NES audio playback
// Receives 44100 Hz f32 audio samples from the main thread via SharedArrayBuffer

let audioBufferStatus;    // Uint32Array: [readPos, writePos]
let audioBufferFloat32;   // Float32Array: ring buffer of samples

class AudioProcessor extends AudioWorkletProcessor {
    constructor(...args) {
        super(...args);
        this.port.onmessage = (e) => {
            audioBufferStatus = e.data.audioBufferStatus;
            audioBufferFloat32 = e.data.audioBufferFloat32;
        };
    }
    process(inputs, outputs, parameters) {
        const output = outputs[0];
        if (!audioBufferStatus || !audioBufferFloat32) {
            return true;
        }
        const bufLen = audioBufferFloat32.length;
        const readPos = audioBufferStatus[0];
        const writePos = audioBufferStatus[1];
        const available = (writePos - readPos + bufLen) % bufLen;
        const needed = output[0].length;

        for (let i = 0; i < needed; i++) {
            if (audioBufferStatus[0] === audioBufferStatus[1]) {
                // Buffer underrun - fill remaining with silence
                for (let j = i; j < needed; j++) {
                    for (let ch = 0; ch < output.length; ch++) {
                        output[ch][j] = 0;
                    }
                }
                break;
            }
            const sample = audioBufferFloat32[audioBufferStatus[0]];
            for (let ch = 0; ch < output.length; ch++) {
                output[ch][i] = sample;
            }
            audioBufferStatus[0] = (audioBufferStatus[0] + 1) % bufLen;
        }
        return true;
    }
}

registerProcessor("audio-processor", AudioProcessor);
