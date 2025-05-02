export default {
  server: {
    port: 3001
  },
  output: {
    minify: false,
  },
  html: {
    template: './src/index.html',
  },
  module: {
    rules: [
      {
        test: /\.(png|jpe?g|gif|svg)$/,
        type: "asset/resource", // Ensures images are emitted as separate files
        parser: {
          dataUrlCondition: {
            maxSize: 0, // Prevents any inlining as Data URL
          },
        },
      },
    ],
  },
};