pub mod shader {
    pub const SPRITE_FRAGMENT_SHADER: &str = r#"
#version 100
precision mediump float;

varying vec2 uv;

uniform sampler2D indexTexture;
uniform sampler2D spriteSheet;
uniform vec2 canvasSize;
uniform vec2 spriteSheetSize;
uniform float spriteSize;

void main() {
    vec2 texCoord = gl_FragCoord.xy / canvasSize;
    vec4 indexColor = texture2D(indexTexture, texCoord);
    
    // Unpack indices from RGBA
    float index1 = indexColor.r * 255.0;
    float index2 = indexColor.g * 255.0;
    float index3 = indexColor.b * 255.0;
    float index4 = indexColor.a * 255.0;
    
    // Determine which index to use based on the fragment position
    float selectedIndex;
    vec2 spriteOffset;
    if (mod(gl_FragCoord.x, 2.0) < 1.0) {
        if (mod(gl_FragCoord.y, 2.0) < 1.0) {
            selectedIndex = index1;
            
        } else {
            selectedIndex = index3;
            
        }
    } else {
        if (mod(gl_FragCoord.y, 2.0) < 1.0) {
            selectedIndex = index2;
            
        } else {
            selectedIndex = index4;

        }
    }

    float spriteY = selectedIndex * spriteSize;
    vec2 spriteUV = (vec2(0.0, spriteY) + fract(texCoord * canvasSize / 2.0)) / spriteSheetSize;
    
    gl_FragColor = texture2D(spriteSheet, spriteUV);
}
"#;
}
