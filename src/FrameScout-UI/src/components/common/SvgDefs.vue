<!--
  Global SVG Asset Library (invisible)
  - 28 个渐变（14 基础 + 14 带坐标实例）在此统一声明
  - Logo 的 7 条路径合并为 #framescout-logo 单一 symbol
  - 必须常驻 DOM（不受 v-if 切换影响），因此放在 App.vue 顶层无条件渲染
-->
<template>
  <svg
    width="0"
    height="0"
    style="position: absolute; width: 0; height: 0; overflow: hidden"
    aria-hidden="true"
  >
    <defs>
      <!-- ===== Base Gradients (14 raw color definitions) ===== -->
      <linearGradient id="grad24">
        <stop offset="0%" stop-color="#821fc6" stop-opacity="1" />
        <stop offset="100%" stop-color="#2493db" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad30">
        <stop offset="0%" stop-color="#8333ff" stop-opacity="1" />
        <stop offset="100%" stop-color="#67e5e5" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad35">
        <stop offset="0%" stop-color="#e5a800" stop-opacity="1" />
        <stop offset="100%" stop-color="#ebff1a" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad37">
        <stop offset="0%" stop-color="#227ec3" stop-opacity="1" />
        <stop offset="100%" stop-color="#2fe9e9" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad39">
        <stop offset="0%" stop-color="#ffee82" stop-opacity="1" />
        <stop offset="100%" stop-color="#ff8000" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad41">
        <stop offset="0%" stop-color="#f9f922" stop-opacity="1" />
        <stop offset="100%" stop-color="#ff6619" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad44">
        <stop offset="0%" stop-color="#bf99ff" stop-opacity="1" />
        <stop offset="100%" stop-color="#abeeee" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad46">
        <stop offset="0%" stop-color="#d3a6f2" stop-opacity="1" />
        <stop offset="100%" stop-color="#d5e8f6" stop-opacity="0.8" />
      </linearGradient>
      <linearGradient id="grad48">
        <stop offset="0%" stop-color="#a9d1ef" stop-opacity="0.8" />
        <stop offset="100%" stop-color="#baf8f8" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad51">
        <stop offset="0%" stop-color="#fee59b" stop-opacity="0.8" />
        <stop offset="100%" stop-color="#f8ffb3" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad54">
        <stop offset="0%" stop-color="#fff5b3" stop-opacity="0.8" />
        <stop offset="100%" stop-color="#ffcc99" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad56">
        <stop offset="0%" stop-color="#ffff80" stop-opacity="1" />
        <stop offset="100%" stop-color="#ffcdcd" stop-opacity="0.8" />
      </linearGradient>
      <linearGradient id="grad58">
        <stop offset="0%" stop-color="#e7eec3" stop-opacity="1" />
        <stop offset="100%" stop-color="#c3e6ee" stop-opacity="1" />
      </linearGradient>
      <linearGradient id="grad61">
        <stop offset="0%" stop-color="#d4f0f7" stop-opacity="0.5" />
        <stop offset="100%" stop-color="#f8f0d3" stop-opacity="0.5" />
      </linearGradient>

      <!-- ===== Instantiated Gradients (14 coordinate-aware copies) ===== -->
      <linearGradient
        id="grad25"
        href="#grad24"
        xlink:href="#grad24"
        x1="184"
        y1="346"
        x2="482.02686"
        y2="348"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad32"
        href="#grad30"
        xlink:href="#grad30"
        x1="272"
        y1="200"
        x2="778"
        y2="200"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad36"
        href="#grad35"
        xlink:href="#grad35"
        x1="152.3334"
        y1="796.60059"
        x2="479.6666"
        y2="567.39941"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad38"
        href="#grad37"
        xlink:href="#grad37"
        x1="492"
        y1="348"
        x2="838"
        y2="348"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad40"
        href="#grad39"
        xlink:href="#grad39"
        x1="515.21161"
        y1="565.11707"
        x2="814.78839"
        y2="774.88293"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad42"
        href="#grad41"
        xlink:href="#grad41"
        x1="489.6843"
        y1="738.24103"
        x2="500.3157"
        y2="859.75873"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad45"
        href="#grad44"
        xlink:href="#grad44"
        x1="272"
        y1="200"
        x2="778"
        y2="200"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad47"
        href="#grad46"
        xlink:href="#grad46"
        x1="183.99664"
        y1="347"
        x2="482.03021"
        y2="347"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad49"
        href="#grad48"
        xlink:href="#grad48"
        x1="492"
        y1="348"
        x2="838"
        y2="348"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad53"
        href="#grad51"
        xlink:href="#grad51"
        x1="152.57378"
        y1="796.94312"
        x2="479.42621"
        y2="567.05688"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad55"
        href="#grad54"
        xlink:href="#grad54"
        x1="516"
        y1="564"
        x2="814"
        y2="776"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad57"
        href="#grad56"
        xlink:href="#grad56"
        x1="489.81357"
        y1="738.22986"
        x2="500.18643"
        y2="859.7699"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad59"
        href="#grad58"
        xlink:href="#grad58"
        x1="501"
        y1="479.64279"
        x2="501"
        y2="522.35724"
        gradientUnits="userSpaceOnUse"
      />
      <linearGradient
        id="grad62"
        href="#grad61"
        xlink:href="#grad61"
        x1="501"
        y1="479.64279"
        x2="501"
        y2="522.35724"
        gradientUnits="userSpaceOnUse"
      />

      <!-- ===== Logo Symbol Definition (7 paths merged into one unit) ===== -->
      <symbol id="framescout-logo" viewBox="0 0 1000 1000">
        <g>
          <path
            style="
              fill: url(#grad25);
              fill-rule: nonzero;
              stroke: url(#grad47);
              stroke-width: 5;
              stroke-dasharray: none;
              stroke-linejoin: round;
              stroke-linecap: round;
            "
            d="M 360,500 150,200 500,260 v 190 z"
          />
          <path
            style="
              fill: url(#grad32);
              fill-opacity: 1;
              fill-rule: nonzero;
              stroke: url(#grad45);
              stroke-opacity: 1;
              stroke-width: 5;
              stroke-dasharray: none;
              stroke-linejoin: round;
              paint-order: normal;
              stroke-linecap: round;
            "
            d="M 500,260 150,200 500,140 850,200 Z"
          />
          <path
            style="
              fill: url(#grad36);
              fill-rule: nonzero;
              stroke: url(#grad53);
              stroke-width: 5;
              stroke-dasharray: none;
              stroke-linejoin: round;
              stroke-linecap: round;
            "
            d="M 360,500 150,800 500,720 V 550 Z"
          />
          <path
            style="
              fill: url(#grad38);
              fill-rule: nonzero;
              stroke: url(#grad49);
              stroke-width: 5;
              stroke-dasharray: none;
              stroke-linejoin: round;
              stroke-linecap: round;
            "
            d="M 640,500 500,450 V 260 l 350,-60 z"
          />
          <path
            style="
              fill: url(#grad40);
              fill-rule: nonzero;
              stroke: url(#grad55);
              stroke-width: 5;
              stroke-dasharray: none;
              stroke-linejoin: round;
              stroke-linecap: round;
            "
            d="M 500,550 640,500 850,800 500,720 Z"
          />
          <path
            style="
              fill: url(#grad42);
              fill-rule: nonzero;
              stroke: url(#grad57);
              stroke-width: 5;
              stroke-dasharray: none;
              stroke-linejoin: round;
              stroke-linecap: round;
            "
            d="m 500,720 -350,80 350,80 350,-80 z"
          />
          <path
            style="
              fill: url(#grad59);
              fill-opacity: 1;
              fill-rule: nonzero;
              stroke: url(#grad62);
              stroke-width: 5;
              stroke-linecap: round;
              stroke-linejoin: round;
              stroke-dasharray: none;
              stroke-opacity: 1;
              paint-order: normal;
            "
            d="m 500,450 -140,50 140,50 140,-50 z"
          />
        </g>
      </symbol>
    </defs>
  </svg>
</template>
