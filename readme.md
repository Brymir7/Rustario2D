# Rustario2D

![Rustario2D Gameplay](./preview.png)

**Rustario2D** is a simple 2D platformer inspired by Mario, developed in Rust as a learning project. It includes an animation system for displaying multiple animations, basic 2D collision handling using dynamic dispatch, and classic Mario-style gameplay.

## Features

- **Animation System**: Supports multiple animations for characters and objects.
- **2D Collisions**: Simple collision detection, including handling for platforms, obstacles, and enemies.
- **Classic Gameplay**: Control the character using arrow keys or WASD to navigate through the level.

## Getting Started

To run the game, use the following commands in the root directory of the project:

cargo run

Or for a release build:

cargo run --release

### Controls

- **Arrow Keys** or **WASD** + **Spacebar**: Move the character left, right, jump.

## Known Limitations

- **End of Game**: The animation system has no animation for end of the game, but they can be easily added.
- **Collision System**: Rarely, collisions can be finicky due to diagonal checking sometimes updating x velocity.
- **Score System**: No score system is implemented.
- **Level Data**: The level data currently lacks information about "Powerup Blocks" (any block with `?`). So every block of this kind is a powerup. Coins are not implemented at all.

## License

This project is licensed under the MIT License.
