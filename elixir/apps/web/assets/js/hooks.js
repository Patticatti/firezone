import { initTabs } from "flowbite";

let Hooks = {};

// Copy to clipboard

Hooks.Tabs = {
  mounted() {
    initTabs();
  },

  updated() {
    initTabs();
  },
};

Hooks.Analytics = {
  mounted() {
    this.handleEvent("identify", ({ id, account_id, name, email }) => {
      var mixpanel = window.mixpanel || null;
      if (mixpanel) {
        mixpanel.identify(id);
        mixpanel.people.set({
          $name: name,
          $email: email,
          account_id: account_id,
        });
        mixpanel.set_group("account", account_id);
      }

      var _hsq = window._hsq || null;
      if (_hsq) {
        _hsq.push(["identify", { id: id, email: email }]);
      }
    });

    this.handleEvent("track_event", ({ name, properties }) => {
      var mixpanel = window.mixpanel || null;
      if (mixpanel) {
        mixpanel.track(name, properties);
      }

      var _hsq = window._hsq || null;
      if (_hsq) {
        _hsq.push([
          "trackCustomBehavioralEvent",
          {
            name: name,
            properties: properties,
          },
        ]);
      }
    });
  },
};

Hooks.Refocus = {
  mounted() {
    this.el.addEventListener("click", (ev) => {
      ev.preventDefault();
      let target_id = ev.currentTarget.getAttribute("data-refocus");
      let el = document.getElementById(target_id);
      if (document.activeElement === el) return;
      el.focus();
    });
  },
};

Hooks.Copy = {
  mounted() {
    this.el.addEventListener("click", (ev) => {
      ev.preventDefault();

      let inner_html = ev.currentTarget
        .querySelector("[data-copy]")
        .innerHTML.trim();
      let doc = new DOMParser().parseFromString(inner_html, "text/html");
      let text = doc.documentElement.textContent;

      let content = ev.currentTarget.querySelector("[data-content]");
      let icon_cl = ev.currentTarget.querySelector("[data-icon]").classList;

      navigator.clipboard.writeText(text).then(() => {
        icon_cl.add("hero-clipboard-document-check");
        icon_cl.add("hover:text-accent-500");
        icon_cl.remove("hero-clipboard-document");
        if (content) {
          content.innerHTML = "Copied";
        }
      });

      setTimeout(() => {
        icon_cl.remove("hero-clipboard-document-check");
        icon_cl.remove("hover:text-accent-500");
        icon_cl.add("hero-clipboard-document");
        if (content) {
          content.innerHTML = "Copy";
        }
      }, 2000);
    });
  },
};

/* The phx-disable-with attribute on submit buttons only applies to liveview forms.
 * However, we need to disable the submit button for regular forms as well to prevent
 * double submissions and cases where the submit handler is slow (e.g. constant-time auth).
 */
Hooks.AttachDisableSubmit = {
  mounted() {
    this.el.addEventListener("form:disable_and_submit", (ev) => {
      let submit = this.el.querySelector('[type="submit"]');
      submit.setAttribute("disabled", "disabled");
      submit.classList.add("cursor-wait");
      submit.classList.add("opacity-75");

      this.el.submit();

      setTimeout(() => {
        submit.classList.remove("cursor-wait");
        submit.classList.remove("opacity-75");
        submit.removeAttribute("disabled");
      }, 5000);
    });
  },

  destroyed() {
    this.el.removeEventListener("form:disable_and_submit");
  },
};

export default Hooks;
