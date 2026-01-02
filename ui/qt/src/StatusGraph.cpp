#include "StatusGraph.h"

#include <QFont>
#include <QLinearGradient>
#include <QPainter>
#include <QPaintEvent>
#include <QPen>
#include <QPolygonF>
#include <algorithm>

StatusGraph::StatusGraph(QWidget* parent)
    : QWidget(parent)
{
    setMinimumHeight(120);
}

void StatusGraph::updateData(const QVector<double>& profile, int detectionCount)
{
    profile_ = profile;
    detection_count_ = detectionCount;
    update();
}

void StatusGraph::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    QLinearGradient gradient(rect().topLeft(), rect().bottomRight());
    gradient.setColorAt(0.0, QColor(22, 22, 22));
    gradient.setColorAt(1.0, QColor(44, 44, 44));
    painter.fillRect(rect(), gradient);

    if (!profile_.isEmpty()) {
        const auto maxValue = *std::max_element(profile_.cbegin(), profile_.cend());
        const qreal height = rect().height();
        const qreal width = rect().width();
        QPolygonF line;
        for (int i = 0; i < profile_.size(); ++i) {
            qreal x = rect().left() + (width - 1.0) * i / qMax(1, profile_.size() - 1);
            qreal normalized = maxValue > 0.0 ? profile_[i] / maxValue : 0.0;
            qreal y = rect().bottom() - normalized * height;
            line.append({x, y});
        }
        QPen signalPen(QColor(0, 190, 255));
        signalPen.setWidthF(2.0);
        painter.setPen(signalPen);
        painter.drawPolyline(line);
    } else {
        painter.setPen(Qt::gray);
        painter.drawText(rect(), Qt::AlignCenter, tr("Awaiting data..."));
    }

    painter.setPen(Qt::white);
    painter.setFont(QFont(painter.font().family(), 10, QFont::Bold));
    painter.drawText(rect().adjusted(12, 10, -12, -10), Qt::AlignTop | Qt::AlignRight,
                     tr("Detections: %1").arg(detection_count_));
}
