let audioBufferStatus;
let audioBufferFloat32;

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
            return false;
        }
        for (let i = 0; i < output[0].length; i++) {
            if (audioBufferStatus[0] == audioBufferStatus[1]) {
                //console.warn("buffer underrun");
                break;
            }
            output[0][i] = audioBufferFloat32[audioBufferStatus[0]];
            audioBufferStatus[0]++;
            audioBufferStatus[0] %= audioBufferFloat32.length;
        }
        return true;
    }
}

registerProcessor("audio-processor", AudioProcessor);