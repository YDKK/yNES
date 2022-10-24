const audioBuffer = new ArrayBuffer(audio_buffer_length * 4 * 3);
const audioBufferFloat32 = new Float32Array(audioBuffer);
let audioBufferFilled = 0;
let audioBufferConsumed = 0;

class AudioProcessor extends AudioWorkletProcessor {
    constructor(...args) {
        super(...args);
        this.port.onmessage = (e) => {
            console.log(e.data);
            for (let i = 0; i < e.data.pcm_filled; i++) {
                audioBufferFloat32[audioBufferFilled] = e.data.audioBufferFloat32[i];
                audioBufferFilled++;
                audioBufferFilled %= audioBufferFloat32.length;
                if (audioBufferFilled == audioBufferConsumed) {
                    console.warn("buffer overrun");
                    audioBufferConsumed++;
                }
            }
            this.port.postMessage("pong");
        };
    }
    process(inputs, outputs, parameters) {
        for (let i = 0; i < outputs.length; i++) {
            outputs[i] = audioBufferFilled[audioBufferConsumed];
            audioBufferConsumed++;
            audioBufferConsumed %= audioBufferFloat32.length;
            if (audioBufferFilled == audioBufferConsumed) {
                console.warn("buffer underrun");
                audioBufferFilled++;
            }
    }
        return true;
    }
}

registerProcessor("audio-processor", AudioProcessor);