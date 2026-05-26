#pragma once
#include <vector>
#include <cassert>

// Asymmetric exponential smoothing filter.
// Uses alpha_rise when signal increases, alpha_decay when it falls.
// Small alpha = more smoothing; large alpha = faster tracking.
class ExpFilter {
public:
    float value;

    ExpFilter(float initial, float alpha_decay, float alpha_rise)
        : value(initial), alpha_decay_(alpha_decay), alpha_rise_(alpha_rise)
    {
        assert(alpha_decay > 0.0f && alpha_decay < 1.0f);
        assert(alpha_rise  > 0.0f && alpha_rise  < 1.0f);
    }

    float update(float input) {
        float alpha = input > value ? alpha_rise_ : alpha_decay_;
        value = alpha * input + (1.0f - alpha) * value;
        return value;
    }

private:
    float alpha_decay_, alpha_rise_;
};

// Array-valued version — applies the same asymmetric rule element-wise.
class ExpFilterArray {
public:
    std::vector<float> value;

    ExpFilterArray(int n, float initial, float alpha_decay, float alpha_rise)
        : value(n, initial), alpha_decay_(alpha_decay), alpha_rise_(alpha_rise)
    {
        assert(n > 0);
        assert(alpha_decay > 0.0f && alpha_decay < 1.0f);
        assert(alpha_rise  > 0.0f && alpha_rise  < 1.0f);
    }

    void update(const float* input, int n) {
        assert(n == static_cast<int>(value.size()));
        for (int i = 0; i < n; ++i) {
            float alpha = input[i] > value[i] ? alpha_rise_ : alpha_decay_;
            value[i] = alpha * input[i] + (1.0f - alpha) * value[i];
        }
    }

    void update(const std::vector<float>& input) {
        update(input.data(), static_cast<int>(input.size()));
    }

private:
    float alpha_decay_, alpha_rise_;
};
