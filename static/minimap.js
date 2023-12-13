// Credit: https://www.stefanjudis.com/a-firefox-only-minimap/ for this awesome minimap
const styles = `
mini-map {
  display: block;
  position: absolute;
  top: 5em;
  right: 1em;
  height: calc(100% - 7em);
  padding: 2em 1em 1em;
}

mini-map .screen-image {
  border-radius: 0.5em;
  box-shadow: var(--soft-shadow-small);
  position: sticky;
  padding: 16px;
  top: 1em;
  bottom: 1em;
}

mini-map .pointer {
  // width: 1.5em;
  height: 1.5em;
  padding: 0.25em;
  border-radius: .375em;
  position: absolute;
  top: 8px;
  right: -.25em;
  left: -.25em;
  transform: translateY(0);
  border: 2px solid rgb(61, 127, 87);
  filter: drop-shadow(0 0 0.125rem #aaa);
}

mini-map .pointer svg {
  fill: white;
  display: block;
  width: 1em;
  height: 100%;
  background: var(--blue-bright);
}

mini-map .screen-image .canvas {
  background: white -moz-element(#main) no-repeat scroll center center / contain;
}

mini-map .screen-image svg {
  position: absolute;
  bottom: -1.25em;
  left: 97%;
  width: 1.75em;
  height: 1.75em;
  fill: var(--c-highlight);;
}

mini-map .screen-image .hint {
  position: absolute;
  bottom: -2.375em;
  left: -0.5em;
  font-size: 0.875em;
  width: max-content;
  text-decoration: none;
}
`;

class MiniMap extends HTMLElement {
    constructor() {
        super();

        this.elementCssIsSupported = CSS.supports(
            'background',
            'white -moz-element(#main)'
        );

        if (this.elementCssIsSupported) {
            const styleElem = document.createElement('style');
            styleElem.innerHTML = styles;
            document.head.appendChild(styleElem);
        }
    }

    removeMap() {
        return this.parentNode.removeChild(this);
    }

    connectedCallback() {
        if (!this.elementCssIsSupported) return this.removeMap();

        const mapContainer = document.getElementById('main');
        const {
            width: containerWidth,
            height: containerHeight,
            top: containerTop,
        } = mapContainer.getBoundingClientRect();

        const topScrollBorder = containerTop + window.scrollY;

        this.innerHTML = `
      <div class="screen-image">
        <div class="pointer"></div>
        <div class="canvas"></div>
      </div>
      `;

        const windowAspectRatio =
            window.visualViewport.height / window.visualViewport.width;
        const containerAspectRatio = containerHeight / containerWidth;

        const mapWidth = 90;
        const mapHeight = Math.floor(mapWidth * containerAspectRatio);
        const mq = window.matchMedia('(min-width: 74em)');

        const isNotEnoughSpace =
            mapHeight + 100 > window.innerHeight || !mq.matches;
        if (isNotEnoughSpace) return this.removeMap();

        mq.addEventListener(
            'change',
            () => {
                console.log('removing');
                if (!mq.matches) this.parentNode.removeChild(this);
            },
            { once: true }
        );

        const map = this.querySelector('.canvas');
        map.style.width = `${mapWidth}px`;
        map.style.height = `${mapHeight}px`;

        const pointer = this.querySelector('.pointer');
        const pointerHeight =
            (mapWidth + 2) *
            windowAspectRatio *
            (window.visualViewport.width / containerWidth);
        pointer.style.height = `${pointerHeight}px`;

        setPointerPosition(window.scrollY);

        window.addEventListener(
            'scroll',
            () => {
                setPointerPosition(window.scrollY);
            },
            {
                passive: true,
            }
        );

        function setPointerPosition(scrollY) {
            const pixelsScrolledIntoMain = window.scrollY - topScrollBorder;
            const scrolledIntoRatio = pixelsScrolledIntoMain / containerHeight;
            const transform = Math.floor(scrolledIntoRatio * mapHeight);

            if (scrolledIntoRatio > 0 && transform < mapHeight - pointerHeight + 16) {
                pointer.style.transform = `translateY(${transform}px)`;
            }
        }
    }
}

customElements.define('mini-map', MiniMap);