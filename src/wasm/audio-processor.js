const audio_buffer_length = 3025;
const audioBuffer = new ArrayBuffer(audio_buffer_length * 4 * 10);
const audioBufferFloat32 = new Float32Array(audioBuffer);
let audioBufferFilled = 0;
let audioBufferConsumed = 0;

class AudioProcessor extends AudioWorkletProcessor {
    constructor(...args) {
        super(...args);
        this.port.onmessage = (e) => {
            for (let i = 0; i < e.data.pcm_filled; i++) {
                audioBufferFloat32[audioBufferFilled] = e.data.audioBufferFloat32[i];
                audioBufferFilled++;
                audioBufferFilled %= audioBufferFloat32.length;
                if (audioBufferFilled == audioBufferConsumed) {
                    console.warn("buffer overrun");
                }
            }
            let diff = audioBufferFilled - audioBufferConsumed;
            let current = diff >= 0 ? diff : audioBufferFloat32.length - diff;
            this.port.postMessage(current);
        };
    }
    process(inputs, outputs, parameters) {
        const output = outputs[0];
        for (let i = 0; i < output[0].length; i++) {
            if (audioBufferFilled == audioBufferConsumed) {
                console.warn("buffer underrun");
                break;
            }
            output[0][i] = audioBufferFloat32[audioBufferConsumed];
            audioBufferConsumed++;
            audioBufferConsumed %= audioBufferFloat32.length;
        }
        let diff = audioBufferFilled - audioBufferConsumed;
        let current = diff >= 0 ? diff : audioBufferFloat32.length - diff;
        this.port.postMessage(current);
        return true;
    }
}

registerProcessor("audio-processor", AudioProcessor);