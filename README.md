# Rust Multi-Registry Package Badges

This project generates static badges for package statistics from various registries, including GitHub Container Registry, Docker Hub, and NPM. It's written in Rust for performance and safety, and uses GitHub Actions to periodically update the badges and GitHub Pages to host them.

## Supported Registries

- GitHub Container Registry
- Docker Hub
- NPM

## Setup

1. Fork this repository.

2. Enable GitHub Pages:
   - Go to your repository settings
   - Scroll down to the "GitHub Pages" section
   - Set the source to the `main` branch and the folder to `/root`

3. Configure the packages you want to track:
   - Open `.github/workflows/update-badges.yml`
   - Edit the `Generate badges` step to include your package details:
     ```yaml
     run: |
       ./target/release/multi-registry-badge-generator github owner1 repo1 package1
       ./target/release/multi-registry-badge-generator dockerhub owner2 repo2
       ./target/release/multi-registry-badge-generator npm package3
     ```

4. Set up the required tokens:
   - For GitHub Container Registry: Create a Personal Access Token with `read:packages` scope
   - Go to your repository settings, then Secrets
   - Add a new repository secret named `GITHUB_TOKEN` with your token as the value

5. Commit and push your changes.

6. (Optional) Adjust the update frequency:
   - Open `.github/workflows/update-badges.yml`
   - Modify the `cron` value to change how often the badges are updated

## Usage

After setting up, your badges will be available at:

```
https://your-username.github.io/rust-multi-registry-package-badges/badges/owner-repo-package-pulls.svg
```

To use the badge in your repository, add the following markdown to your README:

```markdown
![Pulls](https://your-username.github.io/rust-multi-registry-package-badges/badges/owner-repo-package-pulls.svg)
```

Replace `your-username`, `owner`, `repo`, and `package` with the appropriate values.

## How it Works

1. GitHub Actions runs the Rust program periodically (every 6 hours by default).
2. The program fetches the latest stats for each configured package from its respective registry.
3. It generates SVG badges for each package.
4. The updated badges are committed back to the repository.
5. GitHub Pages serves these SVG files, which you can embed in your README or anywhere else.

## Adding New Registries

To add support for a new package registry:

1. Add a new function in `src/main.rs` to fetch stats from the new registry.
2. Update the `match` statement in the `main` function to handle the new registry type.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](file) for details.